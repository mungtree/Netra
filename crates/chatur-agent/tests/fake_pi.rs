//! Integration tests for [`RpcTransport`] against a fake `pi` process.
//!
//! The fake is a tiny single-shot Bash script that speaks the RPC protocol:
//! it reads one request line, emits a canned event sequence, then exits (so
//! its stdio buffer flushes). This exercises real process spawning, strict
//! `\n` framing, and id correlation without needing a model server.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chatur_agent::{AgentSpec, PiSession, RpcTransport};
use chatur_core::CoreError;
use chatur_core::model::AgentEvent;
use chatur_core::traits::{AgentSession, AgentTransport, PromptRequest};
use futures::StreamExt;

const ACCEPT_SCRIPT: &str = r#"#!/usr/bin/env bash
IFS= read -r line
id=$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
printf '{"id":"%s","type":"response","command":"prompt","success":true}\n' "$id"
printf '{"type":"agent_start"}\n'
printf '{"type":"turn_start"}\n'
printf '{"type":"message_update","assistantMessageEvent":{"type":"text_delta","text":"hello"}}\n'
printf '{"type":"turn_end"}\n'
printf '{"type":"agent_end"}\n'
"#;

const REJECT_SCRIPT: &str = r#"#!/usr/bin/env bash
IFS= read -r line
id=$(printf '%s' "$line" | sed -n 's/.*"id":"\([^"]*\)".*/\1/p')
printf '{"id":"%s","type":"response","command":"prompt","success":false,"error":"mock rejection"}\n' "$id"
"#;

/// Writes `body` as an executable script in `dir` and returns its path.
fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write fake pi script");
    let mut perms = std::fs::metadata(&path).expect("stat script").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).expect("chmod script");
    path
}

#[tokio::test]
async fn transport_streams_mapped_events() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_script(dir.path(), "fake_pi.sh", ACCEPT_SCRIPT);
    let spec = AgentSpec::new(&script, dir.path());

    let transport = RpcTransport::spawn(&spec).await.expect("spawn fake pi");
    let stream = transport
        .send_prompt(PromptRequest::new("hi"))
        .await
        .expect("send prompt");
    let events: Vec<AgentEvent> = stream.collect().await;

    assert_eq!(
        events,
        vec![
            AgentEvent::TurnStart,
            AgentEvent::AssistantText {
                text: "hello".to_string()
            },
            AgentEvent::TurnEnd,
        ]
    );
    transport.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn transport_surfaces_rejected_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_script(dir.path(), "fake_pi.sh", REJECT_SCRIPT);
    let spec = AgentSpec::new(&script, dir.path());

    let transport = RpcTransport::spawn(&spec).await.expect("spawn fake pi");
    let result = transport.send_prompt(PromptRequest::new("hi")).await;

    match result {
        Ok(_) => panic!("prompt should have been rejected"),
        Err(CoreError::Agent(message)) => assert_eq!(message, "mock rejection"),
        Err(other) => panic!("expected CoreError::Agent, got {other:?}"),
    }
    transport.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn session_run_collects_output_over_real_process() {
    let dir = tempfile::tempdir().unwrap();
    let script = write_script(dir.path(), "fake_pi.sh", ACCEPT_SCRIPT);
    let spec = AgentSpec::new(&script, dir.path());

    let transport = RpcTransport::spawn(&spec).await.expect("spawn fake pi");
    let session = PiSession::new(Arc::new(transport));
    let output = session.run("hi").await.expect("run session");

    assert_eq!(output.text, "hello");
    session.close().await.expect("close session");
}
