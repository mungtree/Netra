//! Integration tests for the [`Chatur`] facade — full queue-to-completion
//! flow, driven by the mock transport (no `pi` process).

use std::sync::Arc;
use std::time::Duration;

use chatur_agent::MockTransportFactory;
use chatur_api::{Chatur, ChaturConfig};
use chatur_core::ids::{JobId, ProjectId};
use chatur_core::model::JobStatus;

/// A `Chatur` instance backed by a mock transport and a temp data directory.
async fn test_chatur() -> (Chatur, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = ChaturConfig {
        data_dir: dir.path().join("data"),
        log_dir: dir.path().join("logs"),
        ..ChaturConfig::default()
    };
    let chatur = Chatur::start_with_factory(config, Arc::new(MockTransportFactory::default()))
        .await
        .expect("start Chatur");
    (chatur, dir)
}

#[tokio::test]
async fn queues_and_completes_a_job_end_to_end() {
    let (chatur, _dir) = test_chatur().await;

    let project = chatur.add_project("demo", "/tmp/demo").await.unwrap();
    let job_id = chatur
        .queue_job(project, "summarize the code")
        .await
        .unwrap();

    let job = chatur
        .wait_for_job(job_id, Duration::from_secs(5))
        .await
        .unwrap();

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.output.expect("job has output").text, "ok");

    chatur.shutdown().await.unwrap();
}

#[tokio::test]
async fn projects_and_jobs_are_listed() {
    let (chatur, _dir) = test_chatur().await;

    let project = chatur.add_project("p", "/tmp/p").await.unwrap();
    assert_eq!(chatur.list_projects().await.unwrap().len(), 1);

    let job_id = chatur.queue_job(project, "task").await.unwrap();
    chatur
        .wait_for_job(job_id, Duration::from_secs(5))
        .await
        .unwrap();

    let jobs = chatur.list_jobs(project).await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job_id);

    chatur.shutdown().await.unwrap();
}

#[tokio::test]
async fn queueing_for_an_unknown_project_fails() {
    let (chatur, _dir) = test_chatur().await;
    assert!(chatur.queue_job(ProjectId::new(), "x").await.is_err());
    chatur.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelling_an_unknown_job_fails() {
    let (chatur, _dir) = test_chatur().await;
    assert!(chatur.cancel_job(JobId::new()).await.is_err());
    chatur.shutdown().await.unwrap();
}
