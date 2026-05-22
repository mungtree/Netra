//! Interfaces.
//!
//! Every cross-crate dependency in the workspace goes through one of these
//! traits. Concrete implementations live in `chatur-agent`, `chatur-store`,
//! and `chatur-engine`; tests substitute mocks.

mod aggregator;
mod bus;
mod queue;
mod repo;
mod session;
mod sink;
mod support;
mod transport;

pub use aggregator::*;
pub use bus::*;
pub use queue::*;
pub use repo::*;
pub use session::*;
pub use sink::*;
pub use support::*;
pub use transport::*;
