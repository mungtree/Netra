//! `chatur` — the headless command-line interface for Mini ChatUR.
//!
//! This binary exists to prove the `chatur-api` library is fully usable
//! without the Tauri shell, and doubles as an integration-test harness.
//!
//! **P0 scaffold.** It currently only loads and reports configuration; real
//! subcommands (`queue`, `batch`, `logs`) land in P4.

use chatur_api::ChaturConfig;

fn main() {
    let config = ChaturConfig::load_or_default("chatur.toml")
        .unwrap_or_else(|err| panic!("failed to load chatur.toml: {err}"));

    println!("Mini ChatUR — chatur v{}", env!("CARGO_PKG_VERSION"));
    println!("P0 scaffold. Subcommands arrive in P4.");
    println!();
    println!("Effective configuration:");
    println!("  pi binary        : {}", config.pi_binary.display());
    println!("  data dir         : {}", config.data_dir.display());
    println!("  log dir          : {}", config.log_dir.display());
    println!(
        "  concurrency      : global={}, per-project={}",
        config.concurrency.global_max, config.concurrency.per_project_max,
    );
}
