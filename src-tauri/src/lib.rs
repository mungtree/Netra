//! Mini ChatUR — Tauri desktop shell.
//!
//! A thin layer over the [`chatur_api`] library: it owns one running
//! [`Chatur`](chatur_api::Chatur) instance as Tauri managed state, exposes the
//! library's operations as `#[tauri::command]`s (see [`commands`]), and bridges
//! the engine's [`DomainEvent`](chatur_core::traits::DomainEvent) stream to the
//! front-end as a `chatur://event` Tauri event.

mod commands;

use chatur_api::{Chatur, ChaturConfig};
use tauri::{Emitter, Manager};

/// Builds and runs the Mini ChatUR desktop application.
///
/// # Panics
/// Panics if the `Chatur` library instance or the Tauri runtime fails to start
/// — there is nothing useful the shell can do without them.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    // The library reads the same `chatur.toml` the CLI uses; defaults apply
    // when the file is absent.
    let config = ChaturConfig::load_or_default("chatur.toml").unwrap_or_default();
    let chatur = tauri::async_runtime::block_on(Chatur::start(config))
        .expect("failed to start the Chatur library");

    tauri::Builder::default()
        .manage(chatur)
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mini ChatUR");
}
