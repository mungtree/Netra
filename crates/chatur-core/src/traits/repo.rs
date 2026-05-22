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
}

/// Storage for [`Batch`]es and their [`BatchItem`]s.
#[async_trait]
pub trait BatchRepo: Send + Sync {
    /// Inserts a new batch.
    async fn create(&self, batch: &Batch) -> Result<()>;
    /// Fetches a batch by id.
    async fn get(&self, id: BatchId) -> Result<Batch>;
    /// Lists all batches.
    async fn list(&self) -> Result<Vec<Batch>>;
    /// Appends an item to a batch.
    async fn add_item(&self, item: &BatchItem) -> Result<()>;
    /// Lists the items of a batch.
    async fn items(&self, batch_id: BatchId) -> Result<Vec<BatchItem>>;
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
