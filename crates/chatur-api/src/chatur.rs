//! [`Chatur`] — the facade that wires the store, agent pool, and engine.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::stream::BoxStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use chatur_agent::{AgentPool, PiTransportFactory, TransportFactory};
use chatur_chroma::{ChromaHandle, ChromaStatus};
use chatur_core::ids::{BatchId, JobId, ProjectId};
use chatur_core::model::{Batch, BatchBuilder, BatchItem, Job, JobStatus, Project};
use chatur_core::traits::{
    BatchRepo, DomainEvent, EventBus, JobQueue, JobRepo, OutputSink, ProjectRepo,
};
use chatur_core::{CoreError, Result};
use chatur_engine::{
    AggregatorRegistry, BatchExecutor, BroadcastEventBus, JobRunner, OutlinesHttpPlanner,
    RetryPolicy, Scheduler, StructuredPlanner,
};
use chatur_store::{Database, FileLogSink, SqliteJobQueue};

use crate::config::{ChaturConfig, ModelConfig};
use crate::notify;
use crate::planner_supervisor::{PlannerRuntimeConfig, PlannerSupervisor};
use crate::resolver::ProjectSpecResolver;

/// The Mini ChatUR application: one running instance owns the database, the
/// agent pool, the job queue, the event bus, and a background scheduler.
///
/// Construct with [`Chatur::start`]; tear down with [`Chatur::shutdown`].
pub struct Chatur {
    config: ChaturConfig,
    db: Database,
    queue: Arc<dyn JobQueue>,
    bus: BroadcastEventBus,
    scheduler: Arc<Scheduler>,
    batch_executor: Arc<BatchExecutor>,
    shutdown: CancellationToken,
    scheduler_task: Mutex<Option<JoinHandle<()>>>,
    /// Optional ChromaDB integration handle. `Some` iff `config.chromadb.enabled`.
    /// When `None`, no chroma-related code runs and the field consumes only a
    /// pointer-sized Option.
    chroma: Option<ChromaHandle>,
    planner_supervisor: Arc<PlannerSupervisor>,
    /// A small, separate pool used only for one-shot module inference, so the
    /// inference run can't starve (or be starved by) the scheduler's job pool.
    infer_pool: Arc<AgentPool>,
    /// What the durable-queue rehydration did at startup (for the resume banner).
    resume_summary: ResumeSummary,
}

impl Chatur {
    /// Starts an instance backed by real `pi` processes.
    ///
    /// # Errors
    /// Returns an error if the data directory or database cannot be opened.
    pub async fn start(config: ChaturConfig) -> Result<Self> {
        Self::start_with_factory(config, Arc::new(PiTransportFactory)).await
    }

    /// Starts an instance with a caller-supplied transport factory.
    ///
    /// Tests pass a mock factory here to avoid spawning `pi`.
    ///
    /// # Errors
    /// Returns an error if the data directory or database cannot be opened.
    pub async fn start_with_factory(
        config: ChaturConfig,
        factory: Arc<dyn TransportFactory>,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir)
            .await
            .map_err(|e| CoreError::Storage(format!("failed to create data dir: {e}")))?;
        let db = Database::connect(config.data_dir.join("chatur.db")).await?;

        // The in-process queue and scheduler do not survive a restart, so any
        // job left in `Queued` or `Running` from the prior run is orphaned —
        // the engine no longer knows about it, and the UI cannot cancel it.
        // Mark such rows `Cancelled` here so the rest of the app sees a clean
        // slate.
        let resume_summary = reconcile_orphan_jobs(&db).await?;

        let pool = AgentPool::new(
            factory.clone(),
            config.concurrency.global_max,
            config.concurrency.per_project_max,
        );
        // Dedicated single-slot pool for on-demand module inference.
        let infer_pool = Arc::new(AgentPool::new(factory, 1, 1));
        let queue: Arc<dyn JobQueue> = Arc::new(SqliteJobQueue::new(db.pool().clone()));
        // Generously sized: agent turns stream one event per token, and several
        // agents run concurrently. A small buffer would drop (garble) tokens if
        // the Tauri event-forwarding task briefly lags.
        let bus = BroadcastEventBus::new(4096);

        let log_sink: Arc<dyn OutputSink> = Arc::new(FileLogSink::new(&config.log_dir));
        let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
        let event_bus: Arc<dyn EventBus> = Arc::new(bus.clone());
        let interrupt_timeout = if config.timeout.enabled {
            Some(Duration::from_secs(config.timeout.secs))
        } else {
            None
        };
        let tool_timeout = if config.timeout.tool_enabled {
            Some(Duration::from_secs(config.timeout.tool_secs))
        } else {
            None
        };
        let runner = Arc::new(JobRunner::new(
            pool,
            jobs,
            event_bus.clone(),
            vec![log_sink],
            RetryPolicy::default(),
            interrupt_timeout,
            tool_timeout,
        ));

        let chroma = if config.chromadb.enabled {
            let h = ChromaHandle::new(config.chromadb.clone());
            if config.chromadb.auto_start {
                let h2 = h.clone();
                tokio::spawn(async move {
                    // Best-effort: bootstrap, register pi MCP entry, then start.
                    // Failures surface via h2.status() and are visible to the UI.
                    if let Err(e) = chatur_chroma::bootstrap::ensure_venv().await {
                        h2.set_status(ChromaStatus::Error {
                            message: format!("bootstrap: {e}"),
                        })
                        .await;
                        return;
                    }
                    let cfg = h2.config().await;
                    if let Err(e) = chatur_chroma::mcp::register_pi_mcp(&cfg.host, cfg.port) {
                        tracing::warn!("failed to register chroma MCP entry: {e}");
                    }
                    if let Err(e) = chatur_chroma::server::start(&h2).await {
                        tracing::warn!("chroma server start failed: {e}");
                    }
                });
            }
            Some(h)
        } else {
            None
        };

        let resolver = Arc::new(ProjectSpecResolver::new(
            db.projects(),
            config.pi_binary.clone(),
            config.default_model.as_ref().map(ModelConfig::to_model_ref),
            config.agent.clone(),
            chroma.clone(),
            event_bus.clone(),
        ));
        let scheduler = Scheduler::new(
            queue.clone(),
            runner,
            resolver,
            config.concurrency.global_max,
        );

        // Planner Arc is always constructed; when disabled, BatchExecutor
        // bypasses it via `with_planner_enabled(false)` and uses the legacy
        // prompt-only path instead.
        let planner: Arc<dyn StructuredPlanner> =
            Arc::new(OutlinesHttpPlanner::new(config.planner.endpoint.clone()));

        let planner_supervisor = Arc::new(PlannerSupervisor::new());
        if config.planner.enabled && config.planner.autostart {
            let cfg = PlannerRuntimeConfig {
                planner: config.planner.clone(),
                default_model: config.default_model.clone(),
                sidecar_dir: PathBuf::from("planner"),
                python: None,
            };
            let sup = planner_supervisor.clone();
            tokio::spawn(async move {
                if let Err(e) = sup.start(&cfg).await {
                    let msg = format!("planner sidecar start failed: {e}");
                    tracing::error!("{msg}");
                    notify::error("planner", msg);
                }
            });
        }

        // The batch executor enqueues mapped jobs onto the *same* queue the
        // scheduler drains; that is how a batch's jobs actually run.
        let batch_executor = Arc::new(
            BatchExecutor::new(
                queue.clone(),
                Arc::new(db.jobs()),
                Arc::new(db.batches()),
                Arc::new(db.projects()),
                Arc::new(db.templates()),
                Arc::new(bus.clone()),
                Arc::new(AggregatorRegistry::with_defaults()),
                planner,
            )
            .with_planner_enabled(config.planner.enabled),
        );

        let shutdown = CancellationToken::new();
        let task = tokio::spawn(scheduler.clone().run(shutdown.clone()));
        tracing::info!("Chatur started");

        Ok(Self {
            config,
            db,
            queue,
            bus,
            scheduler,
            batch_executor,
            shutdown,
            scheduler_task: Mutex::new(Some(task)),
            chroma,
            planner_supervisor,
            infer_pool,
            resume_summary,
        })
    }

    /// The durable-queue rehydration summary captured at startup, for the UI
    /// resume banner.
    #[must_use]
    pub fn resume_summary(&self) -> ResumeSummary {
        self.resume_summary
    }

    /// Returns the ChromaDB handle if the integration is enabled.
    #[must_use]
    pub fn chroma(&self) -> Option<&ChromaHandle> {
        self.chroma.as_ref()
    }

    /// Returns the planner sidecar supervisor. Tauri uses this to restart
    /// the sidecar after the user saves new model settings.
    #[must_use]
    pub fn planner_supervisor(&self) -> Arc<PlannerSupervisor> {
        self.planner_supervisor.clone()
    }

    /// The effective configuration.
    #[must_use]
    pub fn config(&self) -> &ChaturConfig {
        &self.config
    }

    /// Registers a new project and returns its id.
    ///
    /// # Errors
    /// Returns an error if the project cannot be persisted.
    pub async fn add_project(
        &self,
        name: impl Into<String>,
        root_path: impl Into<PathBuf>,
    ) -> Result<ProjectId> {
        let project = Project::new(name, root_path);
        let id = project.id;
        self.db.projects().create(&project).await?;
        Ok(id)
    }

    /// Lists every registered project.
    ///
    /// # Errors
    /// Returns an error if the projects cannot be read.
    pub async fn list_projects(&self) -> Result<Vec<Project>> {
        self.db.projects().list().await
    }

    /// Fetches one project by id.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if no such project exists.
    pub async fn get_project(&self, id: ProjectId) -> Result<Project> {
        self.db.projects().get(id).await
    }

    /// Infers a set of modules for a project using a one-shot, read-only agent.
    ///
    /// The proposal is **not** persisted — the UI reconciles it against the
    /// project's current modules and saves via
    /// [`update_project_modules`](Self::update_project_modules).
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the project is unknown, or an agent
    /// error if inference fails.
    pub async fn infer_project_modules(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<chatur_core::model::Module>> {
        let project = self.db.projects().get(project_id).await?;
        crate::modules::infer_modules(&project, &self.infer_pool, self.config.pi_binary.clone())
            .await
    }

    /// Replaces a project's module list. An empty `modules` is normalized to the
    /// single default `root` module so a project is never module-less.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the project is unknown.
    pub async fn update_project_modules(
        &self,
        project_id: ProjectId,
        modules: Vec<chatur_core::model::Module>,
    ) -> Result<()> {
        let mut project = self.db.projects().get(project_id).await?;
        project.modules = if modules.is_empty() {
            vec![chatur_core::model::Module::root()]
        } else {
            modules
        };
        self.db.projects().update(&project).await
    }

    /// Queues a job for a project and returns its id.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the project does not exist, or a
    /// storage error if the job cannot be persisted.
    pub async fn queue_job(
        &self,
        project_id: ProjectId,
        prompt: impl Into<String>,
    ) -> Result<JobId> {
        self.queue_job_with_options(project_id, prompt, false).await
    }

    /// Queues a job with explicit options. `use_chromadb` is the per-job opt-in
    /// for ChromaDB MCP usage; it is a no-op when the integration is disabled
    /// or the server is not yet running.
    pub async fn queue_job_with_options(
        &self,
        project_id: ProjectId,
        prompt: impl Into<String>,
        use_chromadb: bool,
    ) -> Result<JobId> {
        // Fail fast if the project is unknown.
        self.db.projects().get(project_id).await?;

        let job = Job::new(project_id, prompt).with_chromadb(use_chromadb);
        let id = job.id;
        self.db.jobs().create(&job).await?;
        self.queue.enqueue(job).await?;
        self.bus.publish(DomainEvent::JobQueued { job_id: id });
        Ok(id)
    }

    /// Fetches one job by id.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if no such job exists.
    pub async fn get_job(&self, id: JobId) -> Result<Job> {
        self.db.jobs().get(id).await
    }

    /// Lists every job belonging to a project.
    ///
    /// # Errors
    /// Returns an error if the jobs cannot be read.
    pub async fn list_jobs(&self, project_id: ProjectId) -> Result<Vec<Job>> {
        self.db.jobs().list_by_project(project_id).await
    }

    /// Cancels a job, whether it is running or still queued.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the job is neither running nor queued.
    pub async fn cancel_job(&self, id: JobId) -> Result<()> {
        // A running job is cancelled via its scheduler token.
        if self.scheduler.cancel_running(id).await.is_ok() {
            return Ok(());
        }
        // Removing it from the in-process queue is best-effort: after a restart
        // the queue is empty, so the row only lives in the database. Treat a
        // `NotFound` here as "already gone from the engine" and proceed to mark
        // the DB row cancelled. Anything else is a real error.
        match self.queue.cancel(id).await {
            Ok(()) => {}
            Err(CoreError::NotFound(_)) => {}
            Err(other) => return Err(other),
        }

        let mut job = self.db.jobs().get(id).await?;
        if job.status.is_terminal() {
            // Already terminal — nothing to do, do not regress state.
            return Ok(());
        }
        job.status = JobStatus::Cancelled;
        job.updated_at = Utc::now();
        self.db.jobs().update(&job).await?;
        Ok(())
    }

    /// Hard-deletes a completed/failed/cancelled job.
    ///
    /// # Errors
    /// Returns [`CoreError::Invalid`] if the job is still `Queued` or `Running`
    /// (cancel it first), or [`CoreError::NotFound`] if no such job exists.
    pub async fn delete_job(&self, id: JobId) -> Result<()> {
        let job = self.db.jobs().get(id).await?;
        if !job.status.is_terminal() {
            return Err(CoreError::Invalid(format!(
                "cannot delete job {id} in status {:?} — cancel it first",
                job.status
            )));
        }
        self.db.jobs().delete(id).await
    }

    /// Hard-deletes a batch and its items.
    pub async fn delete_batch(&self, id: BatchId) -> Result<()> {
        self.db.batches().delete(id).await
    }

    /// Hard-deletes every completed/failed/cancelled job for `project_id`.
    /// Returns the number of rows removed.
    pub async fn clear_completed_jobs(&self, project_id: ProjectId) -> Result<u64> {
        self.db
            .jobs()
            .delete_by_status_in_project(
                project_id,
                &[JobStatus::Completed, JobStatus::Failed, JobStatus::Cancelled],
            )
            .await
    }

    /// Creates a batch that runs `prompts` (a series of prompts) against every
    /// project in `project_ids`, reduced by the `strategy` aggregator.
    ///
    /// The batch and its `prompts × projects` items are persisted but not yet
    /// run — call [`run_batch`](Self::run_batch) with the returned id.
    ///
    /// # Errors
    /// Returns [`CoreError::Invalid`] if `prompts` or `project_ids` is empty,
    /// or [`CoreError::NotFound`] if any project does not exist.
    pub async fn create_batch(
        &self,
        name: impl Into<String>,
        prompts: Vec<String>,
        project_ids: Vec<ProjectId>,
        strategy: impl Into<String>,
    ) -> Result<BatchId> {
        self.create_batch_with_options(name, prompts, project_ids, strategy, false)
            .await
    }

    /// Like [`create_batch`](Self::create_batch) but with explicit options.
    pub async fn create_batch_with_options(
        &self,
        name: impl Into<String>,
        prompts: Vec<String>,
        project_ids: Vec<ProjectId>,
        strategy: impl Into<String>,
        use_chromadb: bool,
    ) -> Result<BatchId> {
        let targets = project_ids
            .into_iter()
            .map(|project_id| BatchTargetSpec {
                project_id,
                module_ids: None,
            })
            .collect();
        self.create_batch_full(name, prompts, targets, strategy, use_chromadb, false, None)
            .await
    }

    /// The fullest batch-creation path: per-target module selection plus a
    /// `global` switch that skips module fanout entirely.
    ///
    /// Each target's `module_ids` selects which of its project's modules to fan
    /// out over (`None` = all). When `global` is set, modules are ignored and
    /// one item is created per `(prompt, target)`.
    ///
    /// # Errors
    /// Returns [`CoreError::Invalid`] if `prompts` or `targets` is empty, or
    /// [`CoreError::NotFound`] if any project does not exist.
    pub async fn create_batch_full(
        &self,
        name: impl Into<String>,
        prompts: Vec<String>,
        targets: Vec<BatchTargetSpec>,
        strategy: impl Into<String>,
        use_chromadb: bool,
        global: bool,
        diff_branch: Option<String>,
    ) -> Result<BatchId> {
        // Fail fast on unknown projects before persisting anything.
        for target in &targets {
            self.db.projects().get(target.project_id).await?;
        }

        let mut builder = BatchBuilder::new(name)
            .strategy(strategy)
            .use_chromadb(use_chromadb);
        if let Some(branch) = diff_branch {
            builder = builder.diff_branch(branch);
        }
        for (index, body) in prompts.into_iter().enumerate() {
            builder = builder.prompt(format!("prompt-{}", index + 1), body);
        }
        for spec in targets {
            let mut target = chatur_core::model::BatchTarget::project(spec.project_id);
            target.module_ids = spec.module_ids;
            builder = builder.target(target);
        }
        let mut batch = builder.build()?;
        batch.global = global;
        let id = batch.id;

        self.db.batches().create(&batch).await?;
        let module_map = self.module_map(&batch).await?;
        for item in batch.materialize(&module_map) {
            self.db.batches().add_item(&item).await?;
        }
        tracing::info!(batch = %id, items = batch.item_count(), global, "batch created");
        Ok(id)
    }

    /// Builds the per-target module lookup [`Batch::materialize`] consumes:
    /// each target project mapped to its current module ids.
    async fn module_map(
        &self,
        batch: &Batch,
    ) -> Result<std::collections::HashMap<ProjectId, Vec<chatur_core::ids::ModuleId>>> {
        let mut map = std::collections::HashMap::new();
        for target in &batch.targets {
            if map.contains_key(&target.project_id) {
                continue;
            }
            let project = self.db.projects().get(target.project_id).await?;
            map.insert(
                target.project_id,
                project.modules.iter().map(|m| m.id).collect(),
            );
        }
        Ok(map)
    }

    /// Starts a batch running in the background and returns immediately.
    ///
    /// Progress and completion arrive on the [`DomainEvent`] stream; poll with
    /// [`get_batch`](Self::get_batch) or block with
    /// [`wait_for_batch`](Self::wait_for_batch).
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if no batch with `id` exists.
    pub async fn run_batch(&self, id: BatchId) -> Result<()> {
        // Fail fast if the batch is unknown.
        self.db.batches().get(id).await?;
        let executor = self.batch_executor.clone();
        tokio::spawn(async move {
            let _ = executor.run(id).await;
        });
        Ok(())
    }

    /// Fetches one batch, including its aggregated result once complete.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if no such batch exists.
    pub async fn get_batch(&self, id: BatchId) -> Result<Batch> {
        self.db.batches().get(id).await
    }

    /// Lists every batch.
    ///
    /// # Errors
    /// Returns an error if the batches cannot be read.
    pub async fn list_batches(&self) -> Result<Vec<Batch>> {
        self.db.batches().list().await
    }

    /// Lists the items (one per `prompt × target`) of a batch.
    ///
    /// # Errors
    /// Returns an error if the items cannot be read.
    pub async fn batch_items(&self, id: BatchId) -> Result<Vec<BatchItem>> {
        self.db.batches().items(id).await
    }

    /// Polls until batch `id` reaches a terminal state or `timeout` elapses.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the batch vanishes, or
    /// [`CoreError::Other`] if `timeout` elapses first.
    pub async fn wait_for_batch(&self, id: BatchId, timeout: Duration) -> Result<Batch> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let batch = self.db.batches().get(id).await?;
            if batch.status.is_terminal() {
                return Ok(batch);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CoreError::Other(format!(
                    "timed out waiting for batch {id}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Subscribes to the live stream of [`DomainEvent`]s.
    #[must_use]
    pub fn subscribe_events(&self) -> BoxStream<'static, DomainEvent> {
        self.bus.subscribe()
    }

    /// Polls until `id` reaches a terminal state or `timeout` elapses.
    ///
    /// # Errors
    /// Returns [`CoreError::NotFound`] if the job vanishes, or
    /// [`CoreError::Other`] if `timeout` elapses first.
    pub async fn wait_for_job(&self, id: JobId, timeout: Duration) -> Result<Job> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let job = self.db.jobs().get(id).await?;
            if job.status.is_terminal() {
                return Ok(job);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(CoreError::Other(format!("timed out waiting for job {id}")));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Stops the scheduler and waits for its loop to exit.
    ///
    /// Jobs already running are left to finish.
    ///
    /// # Errors
    /// Currently infallible, but returns `Result` for forward compatibility.
    pub async fn shutdown(self) -> Result<()> {
        self.shutdown.cancel();
        if let Some(task) = self.scheduler_task.lock().await.take() {
            let _ = task.await;
        }
        if let Some(chroma) = &self.chroma {
            let _ = chatur_chroma::server::stop(chroma).await;
        }
        self.planner_supervisor.shutdown().await;
        tracing::info!("Chatur stopped");
        Ok(())
    }
}

/// One target for [`Chatur::create_batch_full`]: a project plus an optional
/// subset of its modules to fan out over (`None` = all modules).
#[derive(Debug, Clone)]
pub struct BatchTargetSpec {
    /// Project to run against.
    pub project_id: ProjectId,
    /// Selected module ids, or `None` for every module of the project.
    pub module_ids: Option<Vec<chatur_core::ids::ModuleId>>,
}

/// How many times a job may be resumed across restarts before it is failed,
/// so an agent that crashes on every start can't loop forever.
const MAX_RESUME_ATTEMPTS: u32 = 5;

/// What [`reconcile_orphan_jobs`] did at startup, surfaced to the UI resume
/// banner.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct ResumeSummary {
    /// Jobs reset from `Running`/left `Queued` to run again this session.
    pub resumed: usize,
    /// Jobs failed at startup — too many attempts, or their module was deleted.
    pub discarded: usize,
}

/// Rehydrates the durable queue at startup.
///
/// Because the queue is now SQLite-backed, `Queued` rows survive a restart and
/// are picked up directly by `dequeue` — they are left untouched here (only
/// validated against deleted modules). `Running` rows were interrupted by the
/// shutdown, so they are reset to `Queued` (with `attempts` bumped) to resume,
/// unless they have exhausted [`MAX_RESUME_ATTEMPTS`].
async fn reconcile_orphan_jobs(db: &Database) -> Result<ResumeSummary> {
    let jobs = db.jobs();
    let projects = db.projects();
    let mut summary = ResumeSummary::default();

    // Cache project module-id sets so we don't re-fetch per job.
    let mut module_ids: std::collections::HashMap<ProjectId, std::collections::HashSet<_>> =
        std::collections::HashMap::new();
    let mut module_still_exists = |project: &Project, module| {
        module_ids
            .entry(project.id)
            .or_insert_with(|| project.modules.iter().map(|m| m.id).collect())
            .contains(&module)
    };

    // Running jobs were interrupted: requeue or fail them.
    for mut job in jobs.list_by_status(JobStatus::Running).await? {
        if let Some(failure) = orphan_failure(&projects, &job, &mut module_still_exists).await? {
            fail_job(&jobs, &mut job, failure).await?;
            summary.discarded += 1;
            continue;
        }
        if job.attempts >= MAX_RESUME_ATTEMPTS {
            fail_job(
                &jobs,
                &mut job,
                format!("exceeded {MAX_RESUME_ATTEMPTS} resume attempts"),
            )
            .await?;
            summary.discarded += 1;
            continue;
        }
        job.status = JobStatus::Queued;
        job.attempts += 1;
        job.started_at = None;
        job.updated_at = Utc::now();
        jobs.update(&job).await?;
        summary.resumed += 1;
    }

    // Queued jobs are durable; only drop ones whose module was deleted.
    for mut job in jobs.list_by_status(JobStatus::Queued).await? {
        if let Some(failure) = orphan_failure(&projects, &job, &mut module_still_exists).await? {
            fail_job(&jobs, &mut job, failure).await?;
            summary.discarded += 1;
        } else {
            summary.resumed += 1;
        }
    }

    if summary.resumed > 0 || summary.discarded > 0 {
        tracing::info!(
            resumed = summary.resumed,
            discarded = summary.discarded,
            "rehydrated durable queue at startup"
        );
    }
    Ok(summary)
}

/// Returns a failure reason if `job` references a deleted project or module,
/// else `None`.
async fn orphan_failure(
    projects: &chatur_store::SqliteProjectRepo,
    job: &Job,
    module_still_exists: &mut impl FnMut(&Project, chatur_core::ids::ModuleId) -> bool,
) -> Result<Option<String>> {
    let Some(module_id) = job.module_id else {
        // Whole-repo job: only the project must still exist.
        return Ok(match projects.get(job.project_id).await {
            Ok(_) => None,
            Err(CoreError::NotFound(_)) => Some("project was deleted".into()),
            Err(e) => return Err(e),
        });
    };
    match projects.get(job.project_id).await {
        Ok(project) => {
            if module_still_exists(&project, module_id) {
                Ok(None)
            } else {
                Ok(Some(format!("module {module_id} was deleted")))
            }
        }
        Err(CoreError::NotFound(_)) => Ok(Some("project was deleted".into())),
        Err(e) => Err(e),
    }
}

/// Transitions `job` to `Failed`, recording `reason` in its output.
async fn fail_job(
    jobs: &chatur_store::SqliteJobRepo,
    job: &mut Job,
    reason: String,
) -> Result<()> {
    use chatur_core::traits::JobRepo;
    job.status = JobStatus::Failed;
    job.updated_at = Utc::now();
    job.finished_at = Some(Utc::now());
    tracing::warn!(job = %job.id, %reason, "discarded orphan job at startup");
    jobs.update(job).await
}
