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
use chatur_core::ids::{JobId, ProjectId};
use chatur_core::model::{Job, JobStatus, Project};
use chatur_core::traits::{DomainEvent, EventBus, JobQueue, JobRepo, OutputSink, ProjectRepo};
use chatur_core::{CoreError, Result};
use chatur_engine::{BroadcastEventBus, InMemoryJobQueue, JobRunner, RetryPolicy, Scheduler};
use chatur_store::{Database, FileLogSink};

use crate::config::{ChaturConfig, ModelConfig};
use crate::resolver::ProjectSpecResolver;

/// The Mini ChatUR application: one running instance owns the database, the
/// agent pool, the job queue, the event bus, and a background scheduler.
///
/// Construct with [`Chatur::start`]; tear down with [`Chatur::shutdown`].
pub struct Chatur {
    config: ChaturConfig,
    db: Database,
    queue: InMemoryJobQueue,
    bus: BroadcastEventBus,
    scheduler: Arc<Scheduler>,
    shutdown: CancellationToken,
    scheduler_task: Mutex<Option<JoinHandle<()>>>,
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

        let pool = AgentPool::new(
            factory,
            config.concurrency.global_max,
            config.concurrency.per_project_max,
        );
        let queue = InMemoryJobQueue::new();
        let bus = BroadcastEventBus::new(512);

        let log_sink: Arc<dyn OutputSink> = Arc::new(FileLogSink::new(&config.log_dir));
        let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
        let event_bus: Arc<dyn EventBus> = Arc::new(bus.clone());
        let runner = Arc::new(JobRunner::new(
            pool,
            jobs,
            event_bus,
            vec![log_sink],
            RetryPolicy::default(),
        ));

        let resolver = Arc::new(ProjectSpecResolver::new(
            db.projects(),
            config.pi_binary.clone(),
            config.default_model.as_ref().map(ModelConfig::to_model_ref),
        ));
        let scheduler = Scheduler::new(
            queue.clone(),
            runner,
            resolver,
            config.concurrency.global_max,
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
            shutdown,
            scheduler_task: Mutex::new(Some(task)),
        })
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
        // Fail fast if the project is unknown.
        self.db.projects().get(project_id).await?;

        let job = Job::new(project_id, prompt);
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
        // Otherwise it must still be in the queue.
        self.queue.cancel(id).await?;
        let mut job = self.db.jobs().get(id).await?;
        job.status = JobStatus::Cancelled;
        job.updated_at = Utc::now();
        self.db.jobs().update(&job).await?;
        Ok(())
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
        tracing::info!("Chatur stopped");
        Ok(())
    }
}
