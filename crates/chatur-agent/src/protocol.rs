//! The `pi` RPC wire protocol: request encoding and event decoding.
//!
//! See `PLAN.md` §9 and the project memory note for the protocol reference.
//! The decoder is deliberately defensive — `pi`'s RPC surface is version-tied,
//! so unknown messages are ignored rather than treated as errors.

use serde::Serialize;
use serde_json::Value;

use chatur_core::model::AgentEvent;

/// An outgoing RPC request sent to `pi`'s stdin.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum RpcRequest {
    /// Run a turn. The text field is `message` (not `prompt`).
    Prompt {
        /// Correlation id echoed back on the response.
        id: String,
        /// The prompt text.
        message: String,
    },
    /// Abort the in-flight turn.
    Abort {
        /// Correlation id.
        id: String,
    },
}

/// A decoded `{"type":"response",...}` acknowledgement.
#[derive(Debug, Clone)]
pub(crate) struct RpcResponse {
    /// Correlation id, matching the request that triggered it.
    pub id: Option<String>,
    /// Whether the command was accepted.
    pub success: bool,
    /// Failure detail when `success` is false.
    pub error: Option<String>,
}

/// The classification of one decoded protocol line.
pub(crate) enum Incoming {
    /// A command acknowledgement.
    Response(RpcResponse),
    /// A normalized agent event for the active turn.
    Event(AgentEvent),
    /// The agent run finished (`agent_end`).
    TurnComplete,
    /// A recognized-but-irrelevant or unknown message.
    Ignored,
}

/// Decodes one JSON line from `pi`'s stdout.
///
/// Returns `None` only when the line is not valid JSON at all.
pub(crate) fn parse_line(line: &str) -> Option<Incoming> {
    let value: Value = serde_json::from_str(line).ok()?;
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if kind == "response" {
        return Some(Incoming::Response(RpcResponse {
            id: value.get("id").and_then(Value::as_str).map(String::from),
            success: value
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            error: value.get("error").and_then(Value::as_str).map(String::from),
        }));
    }

    Some(match kind {
        "agent_end" => Incoming::TurnComplete,
        _ => match map_event(kind, &value) {
            Some(event) => Incoming::Event(event),
            None => Incoming::Ignored,
        },
    })
}

/// Maps a `pi` event object onto a normalized [`AgentEvent`].
fn map_event(kind: &str, value: &Value) -> Option<AgentEvent> {
    match kind {
        "turn_start" => Some(AgentEvent::TurnStart),
        "turn_end" => Some(AgentEvent::TurnEnd),
        "message_update" => {
            let inner = value.get("assistantMessageEvent")?;
            let delta_kind = inner.get("type").and_then(Value::as_str)?;
            let text = inner
                .get("text")
                .or_else(|| inner.get("delta"))
                .or_else(|| inner.get("content"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            match delta_kind {
                "text_delta" => Some(AgentEvent::AssistantText { text }),
                "thinking_delta" => Some(AgentEvent::Thinking { text }),
                _ => None,
            }
        }
        "tool_execution_start" => Some(AgentEvent::ToolCall {
            name: value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            args: value.get("args").cloned().unwrap_or(Value::Null),
        }),
        "tool_execution_end" => Some(AgentEvent::ToolResult {
            name: value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            is_error: value
                .get("result")
                .and_then(|r| r.get("isError"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        "extension_error" => Some(AgentEvent::Error {
            message: value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("extension error")
                .to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_message_field() {
        let json = serde_json::to_string(&RpcRequest::Prompt {
            id: "r1".to_string(),
            message: "hi".to_string(),
        })
        .unwrap();
        assert!(json.contains(r#""type":"prompt""#));
        assert!(json.contains(r#""message":"hi""#));
        assert!(!json.contains(r#""prompt":"hi""#));
    }

    #[test]
    fn decodes_success_response() {
        let line = r#"{"id":"r1","type":"response","command":"prompt","success":true}"#;
        match parse_line(line).unwrap() {
            Incoming::Response(r) => {
                assert_eq!(r.id.as_deref(), Some("r1"));
                assert!(r.success);
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn decodes_failure_response_with_error() {
        let line = r#"{"type":"response","command":"prompt","success":false,"error":"nope"}"#;
        match parse_line(line).unwrap() {
            Incoming::Response(r) => {
                assert!(!r.success);
                assert_eq!(r.error.as_deref(), Some("nope"));
            }
            _ => panic!("expected response"),
        }
    }

    #[test]
    fn agent_end_signals_turn_complete() {
        assert!(matches!(
            parse_line(r#"{"type":"agent_end"}"#).unwrap(),
            Incoming::TurnComplete
        ));
    }

    #[test]
    fn maps_text_delta_to_assistant_text() {
        let line = r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","text":"hello"}}"#;
        match parse_line(line).unwrap() {
            Incoming::Event(AgentEvent::AssistantText { text }) => assert_eq!(text, "hello"),
            _ => panic!("expected assistant text"),
        }
    }

    #[test]
    fn maps_tool_execution_start() {
        let line = r#"{"type":"tool_execution_start","toolCallId":"t1","toolName":"bash","args":{"command":"ls"}}"#;
        match parse_line(line).unwrap() {
            Incoming::Event(AgentEvent::ToolCall { name, .. }) => assert_eq!(name, "bash"),
            _ => panic!("expected tool call"),
        }
    }

    #[test]
    fn unknown_event_is_ignored_not_an_error() {
        assert!(matches!(
            parse_line(r#"{"type":"queue_update","steering":[]}"#).unwrap(),
            Incoming::Ignored
        ));
    }

    #[test]
    fn non_json_line_returns_none() {
        assert!(parse_line("not json at all").is_none());
    }
}
