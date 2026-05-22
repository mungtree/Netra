//! `chatur-agent` — pi coding-agent process management and RPC transport.
//!
//! Spawns and supervises `pi --mode rpc` processes, frames the JSON-Lines
//! protocol (strict `\n` delimiters — see `PLAN.md` §9), and maps `pi` events
//! onto [`chatur_core`]'s normalized [`AgentEvent`](chatur_core::model::AgentEvent).
//!
//! **P0 scaffold.** Concrete implementations (`PiProcess`, `RpcTransport`,
//! `AgentPool`, `MockTransport`) land in P1.

/// Re-export of the domain crate this layer builds upon.
pub use chatur_core;
