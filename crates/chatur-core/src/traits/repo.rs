//! Persistence interfaces (the Repository pattern).
//!
//! `chatur-store` provides the SQLite-backed implementations; tests use
//! in-memory ones.

use async_trait::async_trait;

use crate::Result;
use crate::ids::{BatchId, JobId, ProjectId, TemplateId};
use crate::model::{Batch, BatchItem, Job, JobStatus, Project, PromptTemplate};

/// Storage for [`Project`]s.
#[async_trait]
pub trait ProjectRepo: Send + Sync {
    /// Inserts a new project.
    async fn create(&self, project: &Project) -> Result<()>;
    /// Fetches a project by id.
    async fn get(&self, id: ProjectId) -> Result<Project>;
    /// Persists changes to an existing project (e.g. its modules).
    async fn update(&self, project: &Project) -> Result<()>;
    /// Lists all projects.
    async fn list(&self) -> Result<Vec<Project>>;
    /// Removes a project.
    async fn delete(&self, id: ProjectId) -> Result<()>;
}

/// Storage for [`Job`]s.
#[async_trait]
pub trait JobRepo: Send + Sync {
    /// Inserts a new job.
    async fn create(&self, job: &Job) -> Result<()>;
    /// Fetches a job by id.
    async fn get(&self, id: JobId) -> Result<Job>;
    /// Persists changes to an existing job.
    async fn update(&self, job: &Job) -> Result<()>;
    /// Lists jobs in a given lifecycle state.
    async fn list_by_status(&self, status: JobStatus) -> Result<Vec<Job>>;
    /// Lists every job belonging to a project.
    async fn list_by_project(&self, project_id: ProjectId) -> Result<Vec<Job>>;
    /// Removes a job by id.
    async fn delete(&self, id: JobId) -> Result<()>;
    /// Removes every job for `project_id` whose status is in `statuses`.
    /// Returns the number of rows deleted.
    async fn delete_by_status_in_project(
        &self,
        project_id: ProjectId,
        statuses: &[JobStatus],
    ) -> Result<u64>;
}

/// Storage for [`Batch`]es and their [`BatchItem`]s.
#[async_trait]
pub trait BatchRepo: Send + Sync {
    /// Inserts a new batch.
    async fn create(&self, batch: &Batch) -> Result<()>;
    /// Fetches a batch by id.
    async fn get(&self, id: BatchId) -> Result<Batch>;
    /// Persists changes to an existing batch (status, result).
    async fn update(&self, batch: &Batch) -> Result<()>;
    /// Lists all batches.
    async fn list(&self) -> Result<Vec<Batch>>;
    /// Appends an item to a batch.
    async fn add_item(&self, item: &BatchItem) -> Result<()>;
    /// Persists changes to an existing batch item (its assigned job).
    async fn update_item(&self, item: &BatchItem) -> Result<()>;
    /// Lists the items of a batch.
    async fn items(&self, batch_id: BatchId) -> Result<Vec<BatchItem>>;
    /// Removes a batch and its items.
    async fn delete(&self, id: BatchId) -> Result<()>;
}

/// Storage for [`PromptTemplate`]s.
#[async_trait]
pub trait TemplateRepo: Send + Sync {
    /// Inserts a new template.
    async fn create(&self, template: &PromptTemplate) -> Result<()>;
    /// Fetches a template by id.
    async fn get(&self, id: TemplateId) -> Result<PromptTemplate>;
    /// Lists all templates.
    async fn list(&self) -> Result<Vec<PromptTemplate>>;
}
