//! Integration tests for the [`Netra`] facade — full queue-to-completion
//! flow, driven by the mock transport (no `pi` process).

use std::sync::Arc;
use std::time::Duration;

use netra_agent::MockTransportFactory;
use netra_api::{Netra, NetraConfig};
use netra_core::ids::{JobId, ProjectId};
use netra_core::model::{BatchStatus, JobStatus};

/// A `Netra` instance backed by a mock transport and a temp data directory.
async fn test_netra() -> (Netra, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = NetraConfig {
        data_dir: dir.path().join("data"),
        log_dir: dir.path().join("logs"),
        ..NetraConfig::default()
    };
    let netra = Netra::start_with_factory(config, Arc::new(MockTransportFactory::default()))
        .await
        .expect("start Netra");
    (netra, dir)
}

#[tokio::test]
async fn queues_and_completes_a_job_end_to_end() {
    let (netra, _dir) = test_netra().await;

    let project = netra.add_project("demo", "/tmp/demo").await.unwrap();
    let job_id = netra
        .queue_job(project, "summarize the code")
        .await
        .unwrap();

    let job = netra
        .wait_for_job(job_id, Duration::from_secs(5))
        .await
        .unwrap();

    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.output.expect("job has output").text, "ok");

    netra.shutdown().await.unwrap();
}

#[tokio::test]
async fn projects_and_jobs_are_listed() {
    let (netra, _dir) = test_netra().await;

    let project = netra.add_project("p", "/tmp/p").await.unwrap();
    assert_eq!(netra.list_projects().await.unwrap().len(), 1);

    let job_id = netra.queue_job(project, "task").await.unwrap();
    netra
        .wait_for_job(job_id, Duration::from_secs(5))
        .await
        .unwrap();

    let jobs = netra.list_jobs(project).await.unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, job_id);

    netra.shutdown().await.unwrap();
}

#[tokio::test]
async fn queueing_for_an_unknown_project_fails() {
    let (netra, _dir) = test_netra().await;
    assert!(netra.queue_job(ProjectId::new(), "x").await.is_err());
    netra.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelling_an_unknown_job_fails() {
    let (netra, _dir) = test_netra().await;
    assert!(netra.cancel_job(JobId::new()).await.is_err());
    netra.shutdown().await.unwrap();
}

#[tokio::test]
async fn runs_a_batch_of_prompts_and_aggregates_the_output() {
    let (netra, _dir) = test_netra().await;
    let project = netra.add_project("demo", "/tmp/demo").await.unwrap();

    let batch_id = netra
        .create_batch(
            "code review",
            vec![
                "find bugs".to_string(),
                "find vulnerabilities".to_string(),
                "find perf issues".to_string(),
            ],
            vec![project],
            "concat",
        )
        .await
        .unwrap();

    netra.run_batch(batch_id).await.unwrap();
    let batch = netra
        .wait_for_batch(batch_id, Duration::from_secs(10))
        .await
        .unwrap();

    assert_eq!(batch.status, BatchStatus::Completed);
    let result = batch.result.expect("completed batch has a result");
    assert_eq!(result.source_count, 3);

    // Three prompts × one project → three batch items.
    assert_eq!(netra.batch_items(batch_id).await.unwrap().len(), 3);
    assert_eq!(netra.list_batches().await.unwrap().len(), 1);

    netra.shutdown().await.unwrap();
}

#[tokio::test]
async fn creating_a_batch_for_an_unknown_project_fails() {
    let (netra, _dir) = test_netra().await;
    assert!(
        netra
            .create_batch("b", vec!["x".to_string()], vec![ProjectId::new()], "concat")
            .await
            .is_err()
    );
    netra.shutdown().await.unwrap();
}
