//! Integration tests for [`BatchExecutor`] — full map → reduce flow, wired to
//! the real SQLite store and the `netra-agent` mock transport (no `pi`).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use netra_agent::{AgentPool, AgentSpec, MockTransportFactory};
use netra_core::Result;
use netra_core::model::{BatchBuilder, BatchStatus, Job, Project};
use netra_core::traits::{BatchRepo, EventBus, JobRepo, ProjectRepo};
use netra_engine::{
    AggregatorRegistry, BatchExecutor, BroadcastEventBus, InMemoryJobQueue, JobRunner, MockPlanner,
    RetryPolicy, Scheduler, SpecResolver,
};
use netra_store::Database;

/// A [`SpecResolver`] that returns one fixed spec for every job.
struct StaticResolver(AgentSpec);

#[async_trait]
impl SpecResolver for StaticResolver {
    async fn resolve(&self, _job: &Job) -> Result<AgentSpec> {
        Ok(self.0.clone())
    }
}

/// Spins up a scheduler draining `queue` in the background, plus the matching
/// [`BatchExecutor`]. Returns the executor and a shutdown token.
async fn harness(
    db: &Database,
    queue: InMemoryJobQueue,
) -> (
    BatchExecutor,
    BroadcastEventBus,
    CancellationToken,
    Arc<MockPlanner>,
) {
    let pool = AgentPool::new(Arc::new(MockTransportFactory::default()), 4, 4);
    let jobs: Arc<dyn JobRepo> = Arc::new(db.jobs());
    let bus = BroadcastEventBus::new(256);
    let runner = Arc::new(JobRunner::new(
        pool,
        jobs,
        Arc::new(bus.clone()),
        Vec::new(),
        RetryPolicy::default(),
        None,
        None,
    ));
    let resolver: Arc<dyn SpecResolver> = Arc::new(StaticResolver(AgentSpec::new("pi", "/tmp/p")));
    let queue: Arc<dyn netra_core::traits::JobQueue> = Arc::new(queue);
    let scheduler = Scheduler::new(queue.clone(), runner, resolver, 4);

    let shutdown = CancellationToken::new();
    tokio::spawn(scheduler.run(shutdown.clone()));

    let planner = Arc::new(MockPlanner::new(serde_json::json!({
        "summary": "mock", "findings": []
    })));
    let executor = BatchExecutor::new(
        queue,
        Arc::new(db.jobs()),
        Arc::new(db.batches()),
        Arc::new(db.projects()),
        Arc::new(db.templates()),
        Arc::new(bus.clone()),
        Arc::new(AggregatorRegistry::with_defaults()),
        planner.clone(),
    )
    .with_poll_interval(Duration::from_millis(10));

    (executor, bus, shutdown, planner)
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
async fn concat_batch_runs_every_prompt_and_aggregates() {
    let (db, project) = db_with_project().await;
    let queue = InMemoryJobQueue::new();
    let (executor, _bus, shutdown, _planner) = harness(&db, queue).await;

    let batch = BatchBuilder::new("review")
        .prompt("a", "find bugs")
        .prompt("b", "find perf issues")
        .prompt("c", "find docs gaps")
        .target_project(project.id)
        .strategy("concat")
        .build()
        .unwrap();
    db.batches().create(&batch).await.unwrap();
    for item in batch.materialize(&std::collections::HashMap::new()) {
        db.batches().add_item(&item).await.unwrap();
    }

    let result = executor.run(batch.id).await.unwrap();

    // Three prompts × one target → three outputs, each the mock's "ok".
    assert_eq!(result.source_count, 3);
    assert_eq!(result.summary.matches("ok").count(), 3);

    // The batch is persisted as Completed with its result attached.
    let stored = db.batches().get(batch.id).await.unwrap();
    assert_eq!(stored.status, BatchStatus::Completed);
    assert!(stored.result.is_some());

    // Each item gained a job, and every job completed.
    let items = db.batches().items(batch.id).await.unwrap();
    assert_eq!(items.len(), 3);
    for item in items {
        let job_id = item.job_id.expect("item has a job");
        assert!(db.jobs().get(job_id).await.is_ok());
    }

    shutdown.cancel();
}

#[tokio::test]
async fn reviewer_batch_runs_a_consolidating_job() {
    let (db, project) = db_with_project().await;
    let queue = InMemoryJobQueue::new();
    let (executor, _bus, shutdown, _planner) = harness(&db, queue).await;

    let batch = BatchBuilder::new("reviewed")
        .prompt("a", "prompt one")
        .prompt("b", "prompt two")
        .target_project(project.id)
        .strategy("reviewer")
        .build()
        .unwrap();
    db.batches().create(&batch).await.unwrap();
    for item in batch.materialize(&std::collections::HashMap::new()) {
        db.batches().add_item(&item).await.unwrap();
    }

    let result = executor.run(batch.id).await.unwrap();

    // Two mapped outputs, then one reviewer job whose "ok" is the summary.
    assert_eq!(result.source_count, 2);
    assert_eq!(result.summary, "ok");
    assert_eq!(
        db.batches().get(batch.id).await.unwrap().status,
        BatchStatus::Completed
    );

    shutdown.cancel();
}

#[tokio::test]
async fn structured_reviewer_chunks_outputs_four_at_a_time() {
    let (db, project) = db_with_project().await;
    let queue = InMemoryJobQueue::new();
    let (executor, _bus, shutdown, planner) = harness(&db, queue).await;

    // Nine prompts × one target → nine map outputs. With the default chunk size
    // of 4 the planner should be invoked three times (4 + 4 + 1).
    let mut builder = BatchBuilder::new("chunked").target_project(project.id);
    for i in 0..9 {
        builder = builder.prompt(format!("p{i}"), format!("prompt {i}"));
    }
    let batch = builder.strategy("structured_reviewer").build().unwrap();
    db.batches().create(&batch).await.unwrap();
    for item in batch.materialize(&std::collections::HashMap::new()) {
        db.batches().add_item(&item).await.unwrap();
    }

    let result = executor.run(batch.id).await.unwrap();

    assert_eq!(result.source_count, 9);
    assert_eq!(planner.call_count(), 3, "9 outputs / chunk 4 = 3 planner calls");
    assert_eq!(
        db.batches().get(batch.id).await.unwrap().status,
        BatchStatus::Completed
    );

    shutdown.cancel();
}

#[tokio::test]
async fn batch_emits_started_and_completed_events() {
    let (db, project) = db_with_project().await;
    let queue = InMemoryJobQueue::new();
    let (executor, bus, shutdown, _planner) = harness(&db, queue).await;
    let mut events = bus.subscribe();

    let batch = BatchBuilder::new("events")
        .prompt("a", "do a thing")
        .target_project(project.id)
        .build()
        .unwrap();
    db.batches().create(&batch).await.unwrap();
    for item in batch.materialize(&std::collections::HashMap::new()) {
        db.batches().add_item(&item).await.unwrap();
    }

    executor.run(batch.id).await.unwrap();

    use netra_core::traits::DomainEvent;
    use futures::StreamExt;
    let mut seen = Vec::new();
    while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(50), events.next()).await
    {
        seen.push(event);
    }
    assert!(
        seen.iter()
            .any(|e| matches!(e, DomainEvent::BatchStarted { .. }))
    );
    assert!(
        seen.iter()
            .any(|e| matches!(e, DomainEvent::BatchCompleted { .. }))
    );

    shutdown.cancel();
}
