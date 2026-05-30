//! Mini ChatUR — Tauri desktop shell.
//!
//! A thin layer over the [`chatur_api`] library: it owns one running
//! [`Chatur`](chatur_api::Chatur) instance as Tauri managed state, exposes the
//! library's operations as `#[tauri::command]`s (see [`commands`]), and bridges
//! the engine's [`DomainEvent`](chatur_core::traits::DomainEvent) stream to the
//! front-end as a `chatur://event` Tauri event.

mod commands;

use std::path::PathBuf;

use chatur_api::{notify, Chatur, ChaturConfig};
use tauri::{Emitter, Manager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Returned to the front-end so users can locate / open the log file.
pub struct LogPaths {
    pub dir: PathBuf,
}

/// Holds the non-blocking writer guard. Dropping it flushes pending log
/// lines, so we keep it alive for the entire app lifetime via managed state.
pub struct LogGuard(#[allow(dead_code)] tracing_appender::non_blocking::WorkerGuard);

fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".chatur")
        .join("logs")
}

fn init_tracing() -> LogGuard {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file_appender = tracing_appender::rolling::daily(&dir, "chatur.log");
    let (nb, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_env("CHATUR_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,chatur=debug,chatur_api=debug,chatur_chroma=debug"));
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(nb)
                .with_ansi(false),
        )
        .try_init();
    LogGuard(guard)
}

/// Builds and runs the Mini ChatUR desktop application.
///
/// # Panics
/// Panics if the `Chatur` library instance or the Tauri runtime fails to start
/// — there is nothing useful the shell can do without them.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_guard = init_tracing();
    tracing::info!("Mini ChatUR starting; log dir = {}", log_dir().display());

    // Install the global UI-notification channel *before* starting the library,
    // so any startup error (e.g. planner sidecar) reaches the front-end.
    let mut notify_rx = notify::init();

    // The library reads the same `chatur.toml` the CLI uses; defaults apply
    // when the file is absent.
    let config = ChaturConfig::load_or_default("chatur.toml").unwrap_or_default();
    let chatur = tauri::async_runtime::block_on(Chatur::start(config))
        .expect("failed to start the Chatur library");

    tauri::Builder::default()
        .manage(chatur)
        .manage(log_guard)
        .setup(|app| {
            // Forward every domain event to the front-end.
            let handle = app.handle().clone();
            let mut events = app.state::<Chatur>().subscribe_events();
            tauri::async_runtime::spawn(async move {
                use futures::StreamExt;
                while let Some(event) = events.next().await {
                    let _ = handle.emit("chatur://event", event);
                }
            });
            // Forward UI notifications.
            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(n) = notify_rx.recv().await {
                    let _ = handle2.emit("chatur://notification", n);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_project,
            commands::list_projects,
            commands::get_project,
            commands::queue_job,
            commands::list_jobs,
            commands::get_job,
            commands::cancel_job,
            commands::delete_job,
            commands::delete_batch,
            commands::clear_completed_jobs,
            commands::create_batch,
            commands::run_batch,
            commands::list_batches,
            commands::get_batch,
            commands::batch_items,
            commands::get_config,
            commands::save_config,
            commands::list_pi_models,
            commands::chroma_status,
            commands::chroma_install,
            commands::chroma_start,
            commands::chroma_stop,
            commands::chroma_restart,
            commands::chroma_list_collections,
            commands::chroma_collection_files,
            commands::chroma_delete_collection,
            commands::chroma_index_project,
            commands::chroma_update_settings,
            commands::chroma_set_enabled,
            commands::chroma_set_embedding_model,
            commands::chroma_drop_and_reindex,
            commands::chroma_query,
            commands::get_log_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mini ChatUR");
}
