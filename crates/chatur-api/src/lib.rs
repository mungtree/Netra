//! `chatur-api` — the public headless library facade for Mini ChatUR.
//!
//! This crate is the single entry point for embedders: the `chatur-cli`
//! binary, the Tauri shell, and any third-party consumer. It wires together
//! `chatur-store`, `chatur-agent`, and `chatur-engine` behind one [`Chatur`]
//! facade and has **no dependency on Tauri** — the library is fully usable on
//! its own.
//!
//! ```no_run
//! # async fn demo() -> chatur_core::Result<()> {
//! use chatur_api::{Chatur, ChaturConfig};
//!
//! let chatur = Chatur::start(ChaturConfig::default()).await?;
//! let project = chatur.add_project("demo", "/path/to/repo").await?;
//! let job = chatur.queue_job(project, "summarize the architecture").await?;
//! # let _ = job;
//! chatur.shutdown().await?;
//! # Ok(())
//! # }
//! ```

mod chatur;
pub mod config;
mod resolver;

pub use chatur_core;

pub use chatur::Chatur;
pub use config::{ChaturConfig, ConcurrencyConfig, ConfigError, ModelConfig};
pub use resolver::ProjectSpecResolver;
