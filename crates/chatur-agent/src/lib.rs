//! `chatur-agent` — pi coding-agent process management and RPC transport.
//!
//! Spawns and supervises `pi --mode rpc` processes, frames the JSON-Lines
//! protocol (strict `\n` delimiters — never a Node-readline-style splitter, see
//! `PLAN.md` §9), and maps `pi` events onto [`chatur_core`]'s normalized
//! [`AgentEvent`](chatur_core::model::AgentEvent).
//!
//! # Layers
//! - [`RpcTransport`] — one `pi` process; send a prompt, stream events.
//! - [`PiSession`] — runs a prompt to completion, collecting an
//!   [`AgentOutput`](chatur_core::model::AgentOutput).
//! - [`AgentPool`] — bounds how many processes run at once.
//! - [`MockTransport`] — an in-process test double, no subprocess.
//!
//! [`MockTransport`] is exported (not test-gated) so other crates can use it in
//! their own tests.

pub mod mock;
mod pool;
mod protocol;
mod session;
mod spec;
mod transport;

pub use chatur_core;
pub use mock::{MockTransport, MockTransportFactory};
pub use pool::{AgentLease, AgentPool, PiTransportFactory, TransportFactory};
pub use session::PiSession;
pub use spec::AgentSpec;
pub use transport::RpcTransport;
