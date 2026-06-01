//! Global UI notification channel.
//!
//! Any code anywhere in the workspace can call [`info`] / [`warn`] / [`error`]
//! to surface a toast in the frontend. Backed by one [`tokio::sync::broadcast`]
//! sender stored in a [`OnceLock`], initialised by the Tauri shell at startup
//! (and forwarded to the front-end as a `netra://notification` event). Calls
//! before init, or in tests/CLI where no init happens, drop silently.

use std::sync::OnceLock;

use serde::Serialize;
use tokio::sync::broadcast;

/// Severity. Renders to a toast colour in the UI.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// One toast.
#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub level: Level,
    /// Short tag for grouping (e.g. `"planner"`, `"chroma"`).
    pub source: String,
    pub message: String,
}

static SENDER: OnceLock<broadcast::Sender<Notification>> = OnceLock::new();

/// Installs the global sender. Idempotent — second call is a no-op.
/// Returns a receiver the caller can forward to the UI.
pub fn init() -> broadcast::Receiver<Notification> {
    let tx = SENDER.get_or_init(|| broadcast::channel(256).0);
    tx.subscribe()
}

/// Subscribe to notifications without (re-)initialising. `None` if `init` has
/// not been called.
#[must_use]
pub fn subscribe() -> Option<broadcast::Receiver<Notification>> {
    SENDER.get().map(broadcast::Sender::subscribe)
}

/// Send a notification. Drops silently if nobody initialised the channel.
pub fn send(level: Level, source: impl Into<String>, message: impl Into<String>) {
    let Some(tx) = SENDER.get() else { return };
    let _ = tx.send(Notification {
        level,
        source: source.into(),
        message: message.into(),
    });
}

pub fn info(source: impl Into<String>, message: impl Into<String>) {
    send(Level::Info, source, message);
}
pub fn warn(source: impl Into<String>, message: impl Into<String>) {
    send(Level::Warn, source, message);
}
pub fn error(source: impl Into<String>, message: impl Into<String>) {
    send(Level::Error, source, message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn send_before_init_is_noop() {
        // The sender is global; just verify no panic.
        error("test", "ignored");
    }

    #[tokio::test]
    async fn round_trip() {
        let mut rx = init();
        info("unit", "hello");
        let n = rx.recv().await.unwrap();
        assert_eq!(n.source, "unit");
        assert_eq!(n.message, "hello");
        assert!(matches!(n.level, Level::Info));
    }
}
