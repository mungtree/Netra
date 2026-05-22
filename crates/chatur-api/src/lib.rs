//! `chatur-api` — the public headless library facade for Mini ChatUR.
//!
//! This crate is the single entry point for embedders: the `chatur-cli`
//! binary, the Tauri shell, and any third-party consumer. It wires together
//! `chatur-store`, `chatur-agent`, and `chatur-engine` behind one `Chatur`
//! facade and has **no dependency on Tauri** — the library is fully usable on
//! its own.
//!
//! **P0 scaffold.** Only [`config`] is implemented; the `Chatur` facade lands
//! in P4.

pub mod config;

pub use chatur_core;
pub use config::{ChaturConfig, ConcurrencyConfig, ConfigError, ModelConfig};
