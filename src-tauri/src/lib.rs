//! NETRA — Tauri desktop shell.
//!
//! A thin layer over the [`netra_api`] library: it owns one running
//! [`Netra`](netra_api::Netra) instance as Tauri managed state, exposes the
//! library's operations as `#[tauri::command]`s (see [`commands`]), and bridges
//! the engine's [`DomainEvent`](netra_core::traits::DomainEvent) stream to the
//! front-end as a `netra://event` Tauri event.

mod commands;

use std::path::PathBuf;

use netra_api::{notify, Netra, NetraConfig};
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
        .join(".netra")
        .join("logs")
}

fn init_tracing() -> LogGuard {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file_appender = tracing_appender::rolling::daily(&dir, "netra.log");
    let (nb, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_env("NETRA_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,netra=debug,netra_api=debug,netra_chroma=debug"));
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

/// Builds and runs the NETRA desktop application.
///
/// # Panics
/// Panics if the `Netra` library instance or the Tauri runtime fails to start
/// — there is nothing useful the shell can do without them.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_guard = init_tracing();
    tracing::info!("NETRA starting; log dir = {}", log_dir().display());

    // Install the global UI-notification channel *before* starting the library,
    // so any startup error (e.g. planner sidecar) reaches the front-end.
    let mut notify_rx = notify::init();

    // The library reads the same `netra.toml` the CLI uses; defaults apply
    // when the file is absent.
    let config = NetraConfig::load_or_default("netra.toml").unwrap_or_default();
    let netra = tauri::async_runtime::block_on(Netra::start(config))
        .expect("failed to start the Netra library");

    tauri::Builder::default()
        .manage(netra)
        .manage(log_guard)
        .setup(|app| {
            // Forward every domain event to the front-end.
            let handle = app.handle().clone();
            let mut events = app.state::<Netra>().subscribe_events();
            tauri::async_runtime::spawn(async move {
                use futures::StreamExt;
                while let Some(event) = events.next().await {
                    let _ = handle.emit("netra://event", event);
                }
            });
            // Forward UI notifications.
            let handle2 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Ok(n) = notify_rx.recv().await {
                    let _ = handle2.emit("netra://notification", n);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::add_project,
            commands::list_projects,
            commands::get_project,
            commands::delete_project,
            commands::queue_job,
            commands::list_jobs,
            commands::get_job,
            commands::cancel_job,
            commands::delete_job,
            commands::delete_batch,
            commands::clear_completed_jobs,
            commands::create_batch,
            commands::list_git_branches,
            commands::run_batch,
            commands::list_batches,
            commands::get_batch,
            commands::batch_items,
            commands::infer_project_modules,
            commands::update_project_modules,
            commands::resume_summary,
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
        .expect("error while running NETRA");
}
