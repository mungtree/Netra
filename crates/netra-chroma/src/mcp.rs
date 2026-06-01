//! Manage the `chroma` MCP server entry inside `~/.pi/agent/config.json`.
//!
//! Idempotent: on first run we back up the existing config to
//! `config.json.netra-bak`. Subsequent runs merge / replace only the
//! `mcpServers.chroma` entry without touching anything else.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::error::ChromaError;

#[must_use]
pub fn pi_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi")
        .join("agent")
        .join("config.json")
}

fn read_config(path: &PathBuf) -> Result<Value, ChromaError> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| ChromaError::Io(path.clone(), e))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(ChromaError::Json)
}

fn write_config(path: &PathBuf, value: &Value) -> Result<(), ChromaError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ChromaError::Io(parent.to_path_buf(), e))?;
    }
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(path, text).map_err(|e| ChromaError::Io(path.clone(), e))
}

fn backup_once(path: &PathBuf) -> Result<(), ChromaError> {
    if !path.exists() {
        return Ok(());
    }
    let bak = path.with_extension("json.netra-bak");
    if bak.exists() {
        return Ok(());
    }
    std::fs::copy(path, &bak).map_err(|e| ChromaError::Io(bak, e))?;
    Ok(())
}

/// Insert/refresh `mcpServers.chroma` so pi launches `chroma-mcp` pointing at
/// our local server. Idempotent.
pub fn register_pi_mcp(host: &str, port: u16) -> Result<(), ChromaError> {
    let path = pi_config_path();
    backup_once(&path)?;
    let mut cfg = read_config(&path)?;
    if !cfg.is_object() {
        return Err(ChromaError::PiConfig(format!(
            "{} root must be an object",
            path.display()
        )));
    }
    let entry = json!({
        "command": "uvx",
        "args": [
            "chroma-mcp",
            "--client-type", "http",
            "--host", host,
            "--port", port.to_string(),
        ],
    });
    let obj = cfg.as_object_mut().unwrap();
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| json!({}));
    if !servers.is_object() {
        return Err(ChromaError::PiConfig(
            "mcpServers must be an object".into(),
        ));
    }
    servers
        .as_object_mut()
        .unwrap()
        .insert("chroma".to_string(), entry);
    write_config(&path, &cfg)
}

/// Remove the `chroma` MCP entry from pi's config. No-op if missing.
pub fn unregister_pi_mcp() -> Result<(), ChromaError> {
    let path = pi_config_path();
    if !path.exists() {
        return Ok(());
    }
    let mut cfg = read_config(&path)?;
    let Some(obj) = cfg.as_object_mut() else {
        return Ok(());
    };
    if let Some(servers) = obj.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        servers.remove("chroma");
    }
    write_config(&path, &cfg)
}

/// Returns true if `mcpServers.chroma` is currently present in pi's config.
#[must_use]
pub fn is_registered() -> bool {
    let path = pi_config_path();
    let Ok(cfg) = read_config(&path) else {
        return false;
    };
    cfg.get("mcpServers")
        .and_then(|s| s.get("chroma"))
        .is_some()
}
