//! Domain data types.
//!
//! These are plain data carriers — no behaviour beyond construction helpers.
//! Behaviour lives behind the [`traits`](crate::traits) interfaces.

mod agent;
mod aggregate;
mod batch;
mod finding;
mod job;
mod project;
mod template;

pub use agent::*;
pub use aggregate::*;
pub use batch::*;
pub use finding::*;
pub use job::*;
pub use project::*;
pub use template::*;
