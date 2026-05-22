//! End-to-end test against the real `pi` binary and a local model server.
//!
//! Ignored by default — it needs `pi` on `PATH` and a reachable model. Run it
//! manually after starting the local llama.cpp server:
//!
//! ```text
//! cargo test -p chatur-agent --test real_pi -- --ignored --nocapture
//! ```

use std::sync::Arc;
use std::time::Duration;

use chatur_agent::{AgentSpec, PiSession, RpcTransport};
use chatur_core::model::ModelRef;
use chatur_core::traits::AgentSession;

#[tokio::test]
#[ignore = "requires `pi` on PATH and a running local model server"]
async fn real_pi_roundtrip() {
    let spec = AgentSpec::new("pi", std::env::temp_dir()).with_model(ModelRef {
        provider: "llamacpp".to_string(),
        model: "qwen3.6-35b-a3b".to_string(),
    });

    let transport = RpcTransport::spawn(&spec)
        .await
        .expect("spawn real pi process");
    let session = PiSession::new(Arc::new(transport));

    let output = tokio::time::timeout(
        Duration::from_secs(180),
        session.run("Reply with exactly the word DONE and nothing else."),
    )
    .await
    .expect("pi did not respond within 180s")
    .expect("pi run failed");

    println!("pi replied: {:?}", output.text);
    assert!(
        output.text.to_uppercase().contains("DONE"),
        "expected the reply to contain DONE, got: {:?}",
        output.text
    );

    session.close().await.expect("close session");
}
