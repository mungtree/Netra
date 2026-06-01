//! Integration tests for the SQLite repositories and the file log sink.

use netra_core::CoreError;
use netra_core::ids::{JobId, ProjectId};
use netra_core::model::{
    AgentEvent, BatchBuilder, BatchStatus, Job, JobStatus, Project, PromptTemplate,
};
use netra_core::traits::{BatchRepo, JobRepo, OutputSink, ProjectRepo, TemplateRepo};
use netra_store::{Database, FileLogSink};

async fn db() -> Database {
    Database::in_memory()
        .await
        .expect("open in-memory database")
}

#[tokio::test]
async fn project_create_get_list_delete() {
    let db = db().await;
    let repo = db.projects();

    let project = Project::new("demo", "/tmp/demo");
    repo.create(&project).await.unwrap();

    let fetched = repo.get(project.id).await.unwrap();
    assert_eq!(fetched.name, "demo");
    assert_eq!(repo.list().await.unwrap().len(), 1);

    repo.delete(project.id).await.unwrap();
    assert!(matches!(
        repo.get(project.id).await,
        Err(CoreError::NotFound(_))
    ));
}

#[tokio::test]
async fn deleting_a_missing_project_is_not_found() {
    let db = db().await;
    assert!(matches!(
        db.projects().delete(ProjectId::new()).await,
        Err(CoreError::NotFound(_))
    ));
}

#[tokio::test]
async fn job_lifecycle_and_status_queries() {
    let db = db().await;
    let project = Project::new("p", "/tmp/p");
    db.projects().create(&project).await.unwrap();

    let jobs = db.jobs();
    let mut job = Job::new(project.id, "do the thing");
    jobs.create(&job).await.unwrap();

    assert_eq!(
        jobs.list_by_status(JobStatus::Queued).await.unwrap().len(),
        1
    );
    assert_eq!(jobs.list_by_project(project.id).await.unwrap().len(), 1);

    job.status = JobStatus::Completed;
    jobs.update(&job).await.unwrap();

    assert!(
        jobs.list_by_status(JobStatus::Queued)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        jobs.list_by_status(JobStatus::Completed)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(jobs.get(job.id).await.unwrap().status, JobStatus::Completed);
}

#[tokio::test]
async fn job_foreign_key_to_project_is_enforced() {
    let db = db().await;
    let orphan = Job::new(ProjectId::new(), "no project exists");
    assert!(db.jobs().create(&orphan).await.is_err());
}

#[tokio::test]
async fn updating_a_missing_job_is_not_found() {
    let db = db().await;
    let project = Project::new("p", "/tmp/p");
    db.projects().create(&project).await.unwrap();
    let job = Job::new(project.id, "never created");
    assert!(matches!(
        db.jobs().update(&job).await,
        Err(CoreError::NotFound(_))
    ));
}

#[tokio::test]
async fn batch_with_items_round_trips() {
    let db = db().await;
    let project = Project::new("p", "/tmp/p");
    db.projects().create(&project).await.unwrap();

    let mut batch = BatchBuilder::new("design review")
        .prompt("file-review", "review {{file}}")
        .target_project(project.id)
        .build()
        .unwrap();
    let repo = db.batches();
    repo.create(&batch).await.unwrap();

    let mut item = batch.materialize(&std::collections::HashMap::new()).pop().unwrap();
    repo.add_item(&item).await.unwrap();

    // The item gains a job assignment; persist it via `update_item`.
    item.job_id = Some(JobId::new());
    repo.update_item(&item).await.unwrap();

    // The batch reaches a terminal state; persist it via `update`.
    batch.status = BatchStatus::Completed;
    repo.update(&batch).await.unwrap();

    let fetched = repo.get(batch.id).await.unwrap();
    assert_eq!(fetched.name, "design review");
    assert_eq!(fetched.status, BatchStatus::Completed);

    let items = repo.items(batch.id).await.unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].job_id.is_some());
}

#[tokio::test]
async fn template_round_trips() {
    let db = db().await;
    let template = PromptTemplate::new("reviewer", "consolidate {{suggestions}}");
    db.templates().create(&template).await.unwrap();

    let fetched = db.templates().get(template.id).await.unwrap();
    assert_eq!(fetched.body, "consolidate {{suggestions}}");
    assert_eq!(db.templates().list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn file_log_sink_writes_one_jsonl_line_per_event() {
    let dir = tempfile::tempdir().unwrap();
    let sink = FileLogSink::new(dir.path());
    let job_id = JobId::new();

    sink.on_event(job_id, &AgentEvent::TurnStart).await.unwrap();
    sink.on_event(
        job_id,
        &AgentEvent::AssistantText {
            text: "hello".to_string(),
        },
    )
    .await
    .unwrap();
    sink.flush(job_id).await.unwrap();

    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let path = dir.path().join(date).join(format!("{job_id}.jsonl"));
    let contents = std::fs::read_to_string(&path).expect("log file should exist");
    let lines: Vec<&str> = contents.lines().collect();

    assert_eq!(lines.len(), 2);
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(value.get("ts").is_some());
        assert!(value.get("event").is_some());
    }
}
