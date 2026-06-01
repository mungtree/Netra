//! `netra-api` — the public headless library facade for NETRA.
//!
//! This crate is the single entry point for embedders: the `netra-cli`
//! binary, the Tauri shell, and any third-party consumer. It wires together
//! `netra-store`, `netra-agent`, and `netra-engine` behind one [`Netra`]
//! facade and has **no dependency on Tauri** — the library is fully usable on
//! its own.
//!
//! ```no_run
//! # async fn demo() -> netra_core::Result<()> {
//! use netra_api::{Netra, NetraConfig};
//!
//! let netra = Netra::start(NetraConfig::default()).await?;
//! let project = netra.add_project("demo", "/path/to/repo").await?;
//! let job = netra.queue_job(project, "summarize the architecture").await?;
//! # let _ = job;
//! netra.shutdown().await?;
//! # Ok(())
//! # }
//! ```

mod netra;
pub mod config;
pub mod modules;
pub mod notify;
mod planner_supervisor;
mod resolver;

pub use netra_chroma;
pub use netra_core;

pub use netra::{BatchTargetSpec, Netra, ResumeSummary};
pub use config::{
    AgentConfig, NetraConfig, ConcurrencyConfig, ConfigError, ModelConfig, PlannerConfig,
    ToolsMode,
};
pub use planner_supervisor::{PlannerError, PlannerRuntimeConfig, PlannerSupervisor};
pub use resolver::ProjectSpecResolver;
