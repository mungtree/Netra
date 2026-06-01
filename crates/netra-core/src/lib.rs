//! `netra-core` — domain model and interfaces for NETRA.
//!
//! This crate is the foundation of the workspace. It holds only **pure types**
//! and **traits** (interfaces). It deliberately performs no process spawning,
//! no database access, and pulls in no async runtime.
//!
//! Every boundary between the higher crates (`netra-agent`, `netra-store`,
//! `netra-engine`, `netra-api`) is expressed as a trait defined here, so any
//! layer can be swapped or mocked.
//!
//! See `PLAN.md` at the repository root for the full architecture.

pub mod error;
pub mod ids;
pub mod model;
pub mod traits;

pub use error::{CoreError, Result};
