//! [`RpcTransport`] — drives a single `pi --mode rpc` process.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use chatur_core::model::AgentEvent;
use chatur_core::traits::{AgentTransport, PromptRequest};
use chatur_core::{CoreError, Result};

use crate::protocol::{Incoming, RpcRequest, RpcResponse, parse_line};
use crate::spec::AgentSpec;

/// State shared between the public handle and the background reader task.
struct Shared {
    /// Outstanding requests awaiting a `response`, keyed by correlation id.
    pending: Mutex<HashMap<String, oneshot::Sender<RpcResponse>>>,
    /// Event sink for the turn currently in flight, if any.
    turn_tx: Mutex<Option<mpsc::UnboundedSender<AgentEvent>>>,
    steer_tx: Mutex<Option<oneshot::Sender<RpcResponse>>>
}

/// A transport over one `pi --mode rpc` child process.
///
/// One transport corresponds to one `pi` conversation; prompts are serial —
/// [`send_prompt`](Self::send_prompt) returns an error if a turn is still
/// running.
pub struct RpcTransport {
    shared: Arc<Shared>,
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Child>,
    reader: Mutex<Option<JoinHandle<()>>>,
}

impl RpcTransport {
    /// Spawns a `pi` process for `spec` and starts reading its event stream.
    ///
    /// # Errors
    /// Returns [`CoreError::Transport`] if the process cannot be spawned or its
    /// stdio handles cannot be captured.
    pub async fn spawn(spec: &AgentSpec) -> Result<Self> {
        let mut command = Command::new(&spec.binary);
        command
            .args(spec.build_args())
            .current_dir(&spec.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command
            .spawn()
            .map_err(|e| CoreError::Transport(format!("failed to spawn pi: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::Transport("pi stdin handle unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Transport("pi stdout handle unavailable".to_string()))?;

        let shared = Arc::new(Shared {
            pending: Mutex::new(HashMap::new()),
            turn_tx: Mutex::new(None),
            steer_tx: Mutex::new(None)
        });
        let reader = tokio::spawn(read_loop(stdout, shared.clone()));

        Ok(Self {
            shared,
            stdin: Mutex::new(Some(stdin)),
            child: Mutex::new(child),
            reader: Mutex::new(Some(reader)),
        })
    }

    /// Serializes `request` as one JSON line and writes it to `pi`'s stdin.
    async fn write_request(&self, request: &RpcRequest) -> Result<()> {
        let mut line = serde_json::to_string(request)?;
        line.push('\n');
        let mut guard = self.stdin.lock().await;
        let stdin = guard
            .as_mut()
            .ok_or_else(|| CoreError::Transport("pi stdin is closed".to_string()))?;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| CoreError::Transport(format!("write to pi failed: {e}")))?;
        stdin
            .flush()
            .await
            .map_err(|e| CoreError::Transport(format!("flush to pi failed: {e}")))?;
        Ok(())
    }

    /// Clears the active-turn sink (e.g. after a rejected prompt).
    async fn clear_turn(&self) {
        self.shared.turn_tx.lock().await.take();
    }
}

#[async_trait]
impl AgentTransport for RpcTransport {
    async fn send_prompt(&self, request: PromptRequest) -> Result<BoxStream<'static, AgentEvent>> {
        let id = Uuid::new_v4().to_string();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        {
            let mut turn = self.shared.turn_tx.lock().await;
            if turn.is_some() {
                return Err(CoreError::Agent(
                    "a turn is already in progress on this transport".to_string(),
                ));
            }
            *turn = Some(event_tx);
        }

        let (resp_tx, resp_rx) = oneshot::channel();
        self.shared.pending.lock().await.insert(id.clone(), resp_tx);

        let outgoing = RpcRequest::Prompt {
            id: id.clone(),
            message: request.message,
        };
        if let Err(err) = self.write_request(&outgoing).await {
            self.shared.pending.lock().await.remove(&id);
            self.clear_turn().await;
            return Err(err);
        }

        match resp_rx.await {
            Ok(resp) if resp.success => {
                tracing::debug!(request_id = %id, "pi accepted prompt");
                Ok(UnboundedReceiverStream::new(event_rx).boxed())
            }
            Ok(resp) => {
                self.clear_turn().await;
                Err(CoreError::Agent(
                    resp.error
                        .unwrap_or_else(|| "pi rejected the prompt".to_string()),
                ))
            }
            Err(_) => {
                self.clear_turn().await;
                Err(CoreError::Transport(
                    "pi exited before acknowledging the prompt".to_string(),
                ))
            }
        }
    }

    
    async fn send_steer(&self, request: PromptRequest) -> Result<()> {
        // Steering requires a active turn
        {
            let turn = self.shared.turn_tx.lock().await;
            if turn.is_none() {
                return Err(CoreError::SteerNoTurn("No active turn to steer".to_string()))
            }
        }
        
        let (resp_tx, resp_rx) = oneshot::channel();
        *self.shared.steer_tx.lock().await = Some(resp_tx);
        let outgoing = RpcRequest::Steer { message: request.message };
        if self.write_request(&outgoing).await.is_err() {
            self.shared.steer_tx.lock().await.take();
            return Err(CoreError::Transport("Failed to write steer".to_string()));
        }
        match resp_rx.await {
            Ok(resp) if resp.success => Ok(()),
            // Response - Not success
            Ok(resp) => Err(CoreError::Agent(resp.error.unwrap_or("Steer failure".to_string()))), 
            Err(err) => Err(CoreError::Transport(err.to_string())),
        }
    }

    async fn abort(&self) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.write_request(&RpcRequest::Abort { id }).await
    }

    async fn shutdown(&self) -> Result<()> {
        // Closing stdin lets pi observe EOF and exit cleanly.
        self.stdin.lock().await.take();
        {
            let mut child = self.child.lock().await;
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        if let Some(handle) = self.reader.lock().await.take() {
            handle.abort();
        }
        Ok(())
    }
}

/// Background task: parse `pi`'s stdout line by line and route messages.
///
/// Framing is strict LF (`\n`) only — `read_until(b'\n', ...)` never splits on
/// U+2028/U+2029, which a Node-readline-style reader would (see `PLAN.md` §9).
async fn read_loop(stdout: ChildStdout, shared: Arc<Shared>) {
    let mut reader = BufReader::new(stdout);
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) => break, // EOF — process exited.
            Ok(_) => {}
            Err(err) => {
                tracing::warn!("error reading pi stdout: {err}");
                break;
            }
        }

        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }

        match parse_line(line) {
            Some(Incoming::Response(resp)) => {
                if let Some(id) = resp.id.clone()
                    && let Some(tx) = shared.pending.lock().await.remove(&id)
                {
                    let _ = tx.send(resp);
                } else if shared.steer_tx.lock().await.is_some() {
                    let _ = shared.steer_tx.lock().await.take().unwrap().send(resp);
                }
            }
            Some(Incoming::Event(event)) => {
                if let Some(tx) = shared.turn_tx.lock().await.as_ref() {
                    let _ = tx.send(event);
                }
            }
            Some(Incoming::TurnComplete) => {
                // Dropping the sender ends the caller's event stream.
                shared.turn_tx.lock().await.take();
            }
            Some(Incoming::Ignored) | None => {}
        }
    }

    // Process ended: fail every outstanding request and end any open turn.
    let mut pending = shared.pending.lock().await;
    for (_, tx) in pending.drain() {
        let _ = tx.send(RpcResponse {
            id: None,
            success: false,
            error: Some("pi process exited".to_string()),
        });
    }
    shared.turn_tx.lock().await.take();
}
