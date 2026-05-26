//! Integration tests for [`JobRunner`] and [`Scheduler`].
//!
//! These wire the real `chatur-store` SQLite repositories and the
//! `chatur-agent` mock transport together — no `pi` process is spawned.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio_util::sync::CancellationToken;

use chatur_agent::{AgentPool, AgentSpec, MockTransport, MockTransportFactory, TransportFactory};
use chatur_core::ids::JobId;
use chatur_core::model::{AgentEvent, Job, JobStatus, Project};
use chatur_core::traits::{AgentTransport, DomainEvent, EventBus, JobQueue, JobRepo, ProjectRepo};
use chatur_core::{CoreError, Result};
use chatur_engine::{
    BroadcastEventBus, InMemoryJobQueue, JobRunner, RetryPolicy, Scheduler, SpecResolver,
};
use chatur_store::Database;

/// A factory that fails its first `fail_first` calls with a transport error.
struct FlakyFactory {
    fail_first: usize,
    calls: AtomicUsize,
}

#[async_trait]
impl TransportFactory for FlakyFactory {
    async fn create(&self, _spec: &AgentSpec) -> Result<Arc<dyn AgentTransport>> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call < self.fail_first {
            Err(CoreError::Transport("transient boom".to_string()))
        } else {
            Ok(Arc::new(MockTransport::replying("recovered")))
        }
    }
}

/// A factory that hands out a single shared [`MockTransport`] whose stream
/// emits one `ToolCall` then hangs forever — used to drive the tool-timeout
/// branch of the runner.
struct StuckToolFactory {
    transport: Arc<MockTransport>,
}

#[async_trait]
impl TransportFactory for StuckToolFactory {
    async fn create(&self, _spec: &AgentSpec) -> Result<Arc<dyn AgentTransport>> {
        Ok(self.transport.clone())
    }
}

/// A [`SpecResolver`] that returns one fixed spec for every job.
struct StaticResolver(AgentSpec);

#[async_trait]
impl SpecResolver for StaticResolver {
    async fn resolve(&self, _job: &Job) -> Result<AgentSpec> {
        Ok(self.0.clone())
    }
}

/// An in-memory database holding one project; returns the database and project.
async fn db_with_project() -> (Database, Project) {
    let db = Database::in_memory().await.expect("open database");
    let project = Project::new("p", "/tmp/p");
    db.projects()
        .create(&project)
        .await
        .expect("create project");
    (db, project)
}

#[tokio::test]
async fn runner_completes_job_and_persists_status() {
    let (db, project) = db_with_project().await;
    let job = Job::new(project.id, "hi");
    db.jobs().create(&job).await.unwrap();

    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus: Arc<dyn EventBus> = Arc::new(BroadcastEventBus::new(64));
    let runner = JobRunner::new(pool, jobs.clone(), bus, Vec::new(), RetryPolicy::default(), None, None);

    let output = runner
        .run(
            job.clone(),
            AgentSpec::new("pi", "/tmp/p"),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(output.text, "ok");
    assert_eq!(jobs.get(job.id).await.unwrap().status, JobStatus::Completed);
}

#[tokio::test]
async fn runner_honors_a_pre_cancelled_token() {
    let (db, project) = db_with_project().await;
    let job = Job::new(project.id, "hi");
    db.jobs().create(&job).await.unwrap();

    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus: Arc<dyn EventBus> = Arc::new(BroadcastEventBus::new(64));
    let runner = JobRunner::new(pool, jobs.clone(), bus, Vec::new(), RetryPolicy::default(), None, None);

    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = runner
        .run(job.clone(), AgentSpec::new("pi", "/tmp/p"), cancel)
        .await;

    assert!(matches!(result, Err(CoreError::Cancelled)));
    assert_eq!(jobs.get(job.id).await.unwrap().status, JobStatus::Cancelled);
}

#[tokio::test]
async fn runner_retries_a_transient_failure() {
    let (db, project) = db_with_project().await;
    let job = Job::new(project.id, "hi");
    db.jobs().create(&job).await.unwrap();

    let factory = Arc::new(FlakyFactory {
        fail_first: 1,
        calls: AtomicUsize::new(0),
    });
    let pool = AgentPool::new(factory, 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus: Arc<dyn EventBus> = Arc::new(BroadcastEventBus::new(64));
    let retry = RetryPolicy {
        max_attempts: 3,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(5),
    };
    let runner = JobRunner::new(pool, jobs.clone(), bus, Vec::new(), retry, None, None);

    let output = runner
        .run(
            job.clone(),
            AgentSpec::new("pi", "/tmp/p"),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    assert_eq!(output.text, "recovered");
    let stored = jobs.get(job.id).await.unwrap();
    assert_eq!(stored.status, JobStatus::Completed);
    assert_eq!(stored.attempts, 2);
}

#[tokio::test]
async fn runner_publishes_lifecycle_events() {
    let (db, project) = db_with_project().await;
    let job = Job::new(project.id, "hi");
    db.jobs().create(&job).await.unwrap();

    let bus = BroadcastEventBus::new(128);
    let mut events = bus.subscribe();

    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let runner = JobRunner::new(
        pool,
        jobs,
        Arc::new(bus.clone()),
        Vec::new(),
        RetryPolicy::default(),
        None,
        None,
    );

    runner
        .run(
            job.clone(),
            AgentSpec::new("pi", "/tmp/p"),
            CancellationToken::new(),
        )
        .await
        .unwrap();

    let mut seen = Vec::new();
    while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(50), events.next()).await
    {
        seen.push(event);
    }
    assert!(
        seen.iter()
            .any(|event| matches!(event, DomainEvent::JobStarted { .. })),
        "expected a JobStarted event",
    );
    assert!(
        seen.iter()
            .any(|event| matches!(event, DomainEvent::JobCompleted { .. })),
        "expected a JobCompleted event",
    );
}

#[tokio::test]
async fn scheduler_drains_the_queue() {
    let (db, project) = db_with_project().await;
    let queue = InMemoryJobQueue::new();
    let mut ids = Vec::new();
    for n in 0..3 {
        let job = Job::new(project.id, format!("job {n}"));
        ids.push(job.id);
        db.jobs().create(&job).await.unwrap();
        queue.enqueue(job).await.unwrap();
    }

    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus: Arc<dyn EventBus> = Arc::new(BroadcastEventBus::new(64));
    let runner = Arc::new(JobRunner::new(
        pool,
        jobs.clone(),
        bus,
        Vec::new(),
        RetryPolicy::default(),
        None,
        None,
    ));
    let resolver: Arc<dyn SpecResolver> = Arc::new(StaticResolver(AgentSpec::new("pi", "/tmp/p")));
    let scheduler = Scheduler::new(queue, runner, resolver, 2);

    let shutdown = CancellationToken::new();
    let handle = tokio::spawn(scheduler.clone().run(shutdown.clone()));

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut all_done = true;
        for id in &ids {
            if jobs.get(*id).await.unwrap().status != JobStatus::Completed {
                all_done = false;
                break;
            }
        }
        if all_done {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "scheduler did not finish all jobs in time",
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    shutdown.cancel();
    handle.await.unwrap();
    assert_eq!(scheduler.running_count().await, 0);
}

#[tokio::test]
async fn cancelling_an_unknown_running_job_is_not_found() {
    let (db, _project) = db_with_project().await;
    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus: Arc<dyn EventBus> = Arc::new(BroadcastEventBus::new(16));
    let runner = Arc::new(JobRunner::new(
        pool,
        jobs,
        bus,
        Vec::new(),
        RetryPolicy::default(),
        None,
        None,
    ));
    let resolver: Arc<dyn SpecResolver> = Arc::new(StaticResolver(AgentSpec::new("pi", "/tmp")));
    let scheduler = Scheduler::new(InMemoryJobQueue::new(), runner, resolver, 1);

    assert!(matches!(
        scheduler.cancel_running(JobId::new()).await,
        Err(CoreError::NotFound(_))
    ));
}

#[tokio::test]
async fn runner_aborts_and_steers_when_a_tool_exceeds_its_budget() {
    let (db, project) = db_with_project().await;
    let job = Job::new(project.id, "do a stuck thing");
    db.jobs().create(&job).await.unwrap();

    // Script: announce a `bash` tool call, then hang — no ToolResult / TurnEnd
    // ever arrives, so the runner's per-tool timer must fire.
    let transport = Arc::new(MockTransport::pending_after(vec![
        AgentEvent::TurnStart,
        AgentEvent::ToolCall {
            name: "bash".to_string(),
            args: serde_json::json!({ "command": "sleep 9999" }),
        },
    ]));
    let factory = Arc::new(StuckToolFactory {
        transport: transport.clone(),
    });
    let pool = AgentPool::new(factory, 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus = BroadcastEventBus::new(128);
    let mut events = bus.subscribe();
    let runner = JobRunner::new(
        pool,
        jobs.clone(),
        Arc::new(bus),
        Vec::new(),
        RetryPolicy::default(),
        // No turn-level interrupt; we want the tool-level one to fire.
        None,
        Some(Duration::from_millis(100)),
    );

    // Cancel the run shortly after the steer has had time to fire so the
    // runner exits cleanly (the mock stream stays pending forever).
    let cancel = CancellationToken::new();
    let cancel_after = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(400)).await;
        cancel_after.cancel();
    });

    let result = runner
        .run(job.clone(), AgentSpec::new("pi", "/tmp/p"), cancel)
        .await;
    assert!(matches!(result, Err(CoreError::Cancelled)));

    // The tool-timeout branch must have aborted the turn and sent a steer
    // explaining what happened.
    assert!(
        transport.abort_count() >= 1,
        "expected transport.abort() to be called on tool timeout",
    );
    let steers = transport.steer_messages();
    assert!(
        steers.iter().any(|m| m.contains("bash") && m.contains("too much")),
        "expected steer message naming the tool and calling out scope; got {steers:?}",
    );

    // The synthetic steer is dispatched to the bus as a Prompt event.
    let mut saw_prompt = false;
    while let Ok(Some(event)) =
        tokio::time::timeout(Duration::from_millis(50), events.next()).await
    {
        if let DomainEvent::JobProgress {
            event: AgentEvent::Prompt { text },
            ..
        } = event
        {
            if text.contains("bash") && text.contains("too much") {
                saw_prompt = true;
                break;
            }
        }
    }
    assert!(saw_prompt, "expected a JobProgress(Prompt) event for the steer");
}
