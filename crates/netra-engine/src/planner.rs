//! [`StructuredPlanner`] — schema-constrained JSON generation.
//!
//! The default impl [`OutlinesHttpPlanner`] talks to the Python `netra-planner`
//! sidecar (FastAPI + `outlines`), which forwards the request to the existing
//! llama.cpp OpenAI-compatible server with a JSON-Schema `response_format`.
//! That guarantees the returned value already conforms to the schema, so
//! [`BatchExecutor::run_structured_reviewer`](crate::BatchExecutor) no longer
//! needs the best-effort `parse_findings` fallback path.

use async_trait::async_trait;
use serde_json::Value;

use netra_core::{CoreError, Result};

/// Generates a JSON value that conforms to a JSON Schema.
#[async_trait]
pub trait StructuredPlanner: Send + Sync {
    /// Returns a value matching `schema` for the given `prompt`.
    async fn generate(&self, prompt: &str, schema: &Value) -> Result<Value>;
}

/// HTTP client for the Python `netra-planner` sidecar.
#[derive(Debug, Clone)]
pub struct OutlinesHttpPlanner {
    base_url: String,
    client: reqwest::Client,
}

impl OutlinesHttpPlanner {
    /// `base_url` is the sidecar's root (e.g. `http://127.0.0.1:8899`).
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("reqwest client"),
        }
    }
}

#[async_trait]
impl StructuredPlanner for OutlinesHttpPlanner {
    async fn generate(&self, prompt: &str, schema: &Value) -> Result<Value> {
        let url = format!("{}/generate", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({ "prompt": prompt, "schema": schema });
        tracing::info!(
            url = %url,
            prompt_len = prompt.len(),
            "planner: POST /generate"
        );
        let started = std::time::Instant::now();
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::Agent(format!("planner request failed: {e}")))?;
        tracing::info!(
            status = %resp.status(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "planner: response received"
        );
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(CoreError::Agent(format!(
                "planner returned {status}: {text}"
            )));
        }
        let payload: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| CoreError::Agent(format!("planner response not JSON: {e}")))?;
        payload
            .get("value")
            .cloned()
            .ok_or_else(|| CoreError::Agent("planner response missing `value`".into()))
    }
}

/// Errors on every call. Used when the planner is disabled in config so that
/// asking for `structured_reviewer` is a loud failure rather than a silent
/// regression.
#[derive(Debug, Clone, Default)]
pub struct NullPlanner;

#[async_trait]
impl StructuredPlanner for NullPlanner {
    async fn generate(&self, _prompt: &str, _schema: &Value) -> Result<Value> {
        Err(CoreError::Agent(
            "structured planner is disabled in config; enable it to use structured_reviewer"
                .into(),
        ))
    }
}

/// Returns a canned value, ignoring inputs. Used by tests and offline harnesses.
#[derive(Debug)]
pub struct MockPlanner {
    value: Value,
    calls: std::sync::Mutex<Vec<(String, Value)>>,
}

impl MockPlanner {
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self {
            value,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl StructuredPlanner for MockPlanner {
    async fn generate(&self, prompt: &str, schema: &Value) -> Result<Value> {
        self.calls
            .lock()
            .unwrap()
            .push((prompt.to_string(), schema.clone()));
        Ok(self.value.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn null_planner_errors() {
        let p = NullPlanner;
        let err = p.generate("x", &json!({})).await.unwrap_err();
        assert!(format!("{err}").contains("disabled"));
    }

    #[tokio::test]
    async fn mock_planner_returns_canned_value_and_counts_calls() {
        let p = MockPlanner::new(json!({"ok": true}));
        let v = p.generate("hi", &json!({"type":"object"})).await.unwrap();
        assert_eq!(v, json!({"ok": true}));
        assert_eq!(p.call_count(), 1);
    }

    /// Minimal HTTP server that responds 200 with `{"value": {...}}` to any
    /// POST. We don't pull in `wiremock` just for this.
    async fn run_fake_sidecar(listener: TcpListener, body: String) {
        if let Ok((mut sock, _)) = listener.accept().await {
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    }

    #[tokio::test]
    async fn http_planner_parses_value_field() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body = r#"{"value":{"summary":"ok","findings":[]}}"#.to_string();
        let server = tokio::spawn(run_fake_sidecar(listener, body));

        let p = OutlinesHttpPlanner::new(format!("http://{addr}"));
        let v = p
            .generate("hi", &json!({"type":"object"}))
            .await
            .expect("ok");
        assert_eq!(v["summary"], "ok");
        let _ = server.await;
    }
}
