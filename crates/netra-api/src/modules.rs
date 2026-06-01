//! Module inference: ask a read-only agent to split a repo into modules.
//!
//! The agent receives a shallow directory listing and returns a JSON proposal.
//! Inference is **stateless** — nothing is persisted. The caller (UI) reconciles
//! the proposal against the project's current modules and saves on "Apply".

use std::path::{Path, PathBuf};

use netra_agent::{AgentPool, AgentSpec};
use netra_core::model::{Module, Project, ToolPolicy};
use netra_core::traits::AgentSession;
use netra_core::{CoreError, Result};
use serde::Deserialize;

/// How deep the directory walk goes when building the listing for the agent.
const WALK_DEPTH: usize = 2;

/// Shape the agent is asked to return (one entry per proposed module).
#[derive(Debug, Deserialize)]
struct ProposedModule {
    name: String,
    #[serde(default)]
    description: String,
    root_subdir: String,
}

#[derive(Debug, Deserialize)]
struct InferResponse {
    modules: Vec<ProposedModule>,
}

/// Infers a set of [`Module`]s for `project` using a one-shot, read-only agent.
///
/// The returned modules carry fresh ids and only include entries whose
/// `root_subdir` actually exists under [`Project::root_path`]. The result is
/// **not** persisted.
///
/// # Errors
/// Returns an error if the agent run fails or its output can't be parsed into
/// the expected JSON shape.
pub async fn infer_modules(
    project: &Project,
    pool: &AgentPool,
    pi_binary: PathBuf,
) -> Result<Vec<Module>> {
    let listing = directory_listing(&project.root_path, WALK_DEPTH);

    let spec = AgentSpec::new(pi_binary, project.root_path.clone())
        .with_tool_policy(ToolPolicy::ReadOnly)
        .with_system_prompt_append(SYSTEM_PROMPT.to_string());

    let prompt = format!(
        "Directory listing of `{}` (depth {WALK_DEPTH}, file counts in parens):\n\n{listing}\n\n\
         Return ONLY the JSON object described — no prose, no markdown fences.",
        project.root_path.display()
    );

    let lease = pool.acquire(project.id, spec).await?;
    let output = lease.session().run(&prompt).await;
    let _ = lease.release().await;
    let output = output?;

    let response = parse_response(&output.text)?;
    Ok(validate(response, &project.root_path))
}

/// System prompt establishing the splitter role and output contract.
const SYSTEM_PROMPT: &str = "You are a codebase splitter. Given a directory \
    listing, return a JSON object `{\"modules\":[{\"name\",\"description\",\
    \"root_subdir\"}]}` describing large, generic modules (e.g. frontend / \
    backend / engine / shared). Prefer 3-8 modules. `root_subdir` is a path \
    relative to the repo root. Never split below top-level directories unless a \
    top-level directory clearly contains multiple separable products. Respond \
    with ONLY the JSON object.";

/// Best-effort parse of the agent's text into [`InferResponse`]. Tolerates
/// fenced code blocks and surrounding prose.
fn parse_response(text: &str) -> Result<InferResponse> {
    if let Ok(r) = serde_json::from_str::<InferResponse>(text) {
        return Ok(r);
    }
    let stripped = strip_code_fence(text);
    if let Ok(r) = serde_json::from_str::<InferResponse>(stripped) {
        return Ok(r);
    }
    if let Some(obj) = extract_json_object(stripped) {
        if let Ok(r) = serde_json::from_str::<InferResponse>(obj) {
            return Ok(r);
        }
    }
    Err(CoreError::Agent(format!(
        "module inference agent returned unparseable output: {text}"
    )))
}

/// Drops proposals whose `root_subdir` does not exist under `root`, assigns
/// fresh ids, and normalizes the subdir into a [`PathBuf`].
fn validate(response: InferResponse, root: &Path) -> Vec<Module> {
    response
        .modules
        .into_iter()
        .filter_map(|m| {
            let subdir = PathBuf::from(m.root_subdir.trim_start_matches(['/', '\\']));
            // Empty subdir = whole repo, always valid. Otherwise it must exist.
            if !subdir.as_os_str().is_empty() && !root.join(&subdir).is_dir() {
                return None;
            }
            Some(Module {
                id: netra_core::ids::ModuleId::new(),
                name: m.name,
                description: m.description,
                root_subdir: subdir,
            })
        })
        .collect()
}

/// Builds a compact `dir (N files)` listing of `root`, up to `depth` levels
/// deep. Hidden entries and common noise dirs are skipped.
fn directory_listing(root: &Path, depth: usize) -> String {
    let mut lines = Vec::new();
    walk(root, root, depth, &mut lines);
    if lines.is_empty() {
        "(empty)".to_string()
    } else {
        lines.join("\n")
    }
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || is_noise(&name) {
            continue;
        }
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    for path in dirs {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let count = std::fs::read_dir(&path)
            .map(|e| e.flatten().filter(|e| e.path().is_file()).count())
            .unwrap_or(0);
        out.push(format!("{} ({count} files)", rel.display()));
        walk(root, &path, depth - 1, out);
    }
}

fn is_noise(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "target" | "dist" | "build" | "__pycache__" | "vendor"
    )
}

fn strip_code_fence(text: &str) -> &str {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    t.trim_end_matches("```").trim()
}

fn extract_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let start = bytes.iter().position(|b| *b == b'{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&text[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_plain_and_fenced_json() {
        let plain = r#"{"modules":[{"name":"a","description":"d","root_subdir":"src"}]}"#;
        assert_eq!(parse_response(plain).unwrap().modules.len(), 1);

        let fenced = format!("here:\n```json\n{plain}\n```\nthanks");
        assert_eq!(parse_response(&fenced).unwrap().modules.len(), 1);
    }

    #[test]
    fn validate_drops_missing_subdirs_and_keeps_existing() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("server")).unwrap();
        let response = InferResponse {
            modules: vec![
                ProposedModule {
                    name: "backend".into(),
                    description: "d".into(),
                    root_subdir: "server".into(),
                },
                ProposedModule {
                    name: "ghost".into(),
                    description: "d".into(),
                    root_subdir: "does-not-exist".into(),
                },
            ],
        };
        let modules = validate(response, dir.path());
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name, "backend");
        assert_eq!(modules[0].root_subdir, PathBuf::from("server"));
    }

    #[test]
    fn listing_skips_noise_dirs() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::create_dir(dir.path().join("node_modules")).unwrap();
        let listing = directory_listing(dir.path(), 2);
        assert!(listing.contains("src"));
        assert!(!listing.contains("node_modules"));
    }
}
