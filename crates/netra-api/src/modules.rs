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
use netra_engine::StructuredPlanner;
use serde::Deserialize;

/// JSON wrapper format for module import/export files.
const MODULES_FORMAT: &str = "netra.modules/v1";

/// How deep the directory walk goes when building the listing for the agent.
const WALK_DEPTH: usize = 2;

/// Dirs with more than this many immediate subdirs are collapsed into a single
/// summary line instead of expanded, to keep the prompt small for monorepos.
const MAX_CHILDREN_LISTED: usize = 25;

/// Hard cap on total listing lines. Once hit, the walk stops and appends a
/// truncation marker. Keeps the prompt within the model's context for very
/// large applications.
const MAX_LISTING_LINES: usize = 400;

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

/// Infers modules for `project`, working **incrementally**: the project's
/// current modules are shown to the agent, which is asked to propose only the
/// *additional* modules it discovers. The result is `current ∪ new` — existing
/// modules (and their ids) are preserved, never dropped. The result is **not**
/// persisted.
///
/// New modules carry fresh ids and only include entries whose `root_subdir`
/// actually exists under [`Project::root_path`] and isn't already covered by a
/// current module.
///
/// # Errors
/// Returns an error if the agent run fails or its output can't be parsed into
/// the expected JSON shape.
pub async fn infer_modules(
    project: &Project,
    pool: &AgentPool,
    pi_binary: PathBuf,
) -> Result<Vec<Module>> {
    let spec = AgentSpec::new(pi_binary, project.root_path.clone())
        .with_tool_policy(ToolPolicy::ReadOnly)
        .with_system_prompt_append(SYSTEM_PROMPT.to_string());

    let prompt = build_infer_prompt(project);

    let lease = pool.acquire(project.id, spec).await?;
    let output = lease.session().run(&prompt).await;
    let _ = lease.release().await;
    let output = output?;

    let response = parse_response(&output.text)?;
    let new = validate(response, &project.root_path);
    Ok(merge(&project.modules, new))
}

/// Like [`infer_modules`], but routes generation through the structured planner
/// sidecar (schema-constrained), so the returned JSON is guaranteed to match the
/// expected shape — no best-effort text parsing. Used when the "Structured
/// Output Handler" setting is enabled. The result is **not** persisted.
///
/// # Errors
/// Returns an error if the planner request fails or its value cannot be
/// deserialized into the expected shape.
pub async fn infer_modules_structured(
    project: &Project,
    planner: &dyn StructuredPlanner,
) -> Result<Vec<Module>> {
    let prompt = build_infer_prompt(project);
    let value = planner.generate(&prompt, &infer_schema()).await?;
    let response: InferResponse = serde_json::from_value(value).map_err(|e| {
        CoreError::Agent(format!("planner returned an unexpected module shape: {e}"))
    })?;
    let new = validate(response, &project.root_path);
    Ok(merge(&project.modules, new))
}

/// The prompt asking for additional modules, shared by both the agent and the
/// structured-planner inference paths.
fn build_infer_prompt(project: &Project) -> String {
    format!(
        "Directory of `{}`\n{listing}\n \
         The project ALREADY has these modules:\n{existing}\n\n\
         Propose ONLY ADDITIONAL modules not already covered above — do not \
         repeat or restate the existing ones. If nothing new is worth adding, \
         return an empty `modules` array. Return ONLY the JSON object described \
         — no prose, no markdown fences.",
        project.root_path.display(),
        listing = directory_listing(&project.root_path, WALK_DEPTH),
        existing = format_existing(&project.modules),
    )
}

/// JSON Schema the planner sidecar constrains generation to.
fn infer_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["modules"],
        "properties": {
            "modules": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["name", "description", "root_subdir"],
                    "properties": {
                        "name": { "type": "string" },
                        "description": { "type": "string" },
                        "root_subdir": { "type": "string" }
                    }
                }
            }
        }
    })
}

/// Serializes `modules` to the `netra.modules/v1` export format. Module ids are
/// intentionally omitted — they are local UUIDs reassigned on import.
#[must_use]
pub fn modules_to_json(modules: &[Module]) -> String {
    let body = serde_json::json!({
        "format": MODULES_FORMAT,
        "modules": modules.iter().map(|m| serde_json::json!({
            "name": m.name,
            "description": m.description,
            "root_subdir": m.root_subdir.to_string_lossy(),
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".to_string())
}

/// Parses a `netra.modules/v1` file into modules with fresh ids, dropping any
/// whose `root_subdir` doesn't exist under `root` (same validation as inference).
/// The result is **not** persisted.
///
/// # Errors
/// Returns [`CoreError::Invalid`] if the text is not JSON, the `format` does not
/// match, or the `modules` field is missing/malformed.
pub fn modules_from_json(text: &str, root: &Path) -> Result<Vec<Module>> {
    #[derive(Deserialize)]
    struct ModulesFile {
        #[serde(default)]
        format: String,
        modules: Vec<ProposedModule>,
    }
    let file: ModulesFile = serde_json::from_str(text)
        .map_err(|e| CoreError::Invalid(format!("invalid modules JSON: {e}")))?;
    if file.format != MODULES_FORMAT {
        return Err(CoreError::Invalid(format!(
            "unsupported format \"{}\" — expected \"{MODULES_FORMAT}\"",
            file.format
        )));
    }
    Ok(validate(
        InferResponse {
            modules: file.modules,
        },
        root,
    ))
}

/// System prompt establishing the splitter role and output contract.
const SYSTEM_PROMPT: &str = "You are a codebase splitter. Given a directory \
    listing and the modules a project already has, return a JSON object \
    `{\"modules\":[{\"name\",\"description\",\"root_subdir\"}]}` listing \
    ADDITIONAL modules/components of a project that are not already covered. \
    `root_subdir` is a path relative to the repo root. \
    Never split below top-level directories unless a top-level \
    directory clearly contains multiple separable products. \
    Respond with ONLY the JSON object.";

/// Renders the current modules as a compact bullet list for the prompt.
fn format_existing(modules: &[Module]) -> String {
    if modules.is_empty() {
        return "(none)".to_string();
    }
    modules
        .iter()
        .map(|m| {
            let sub = m.root_subdir.display();
            let sub = if m.root_subdir.as_os_str().is_empty() {
                ". (whole repo)".to_string()
            } else {
                sub.to_string()
            };
            format!("- {} [{}] — {}", m.name, sub, m.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Merges agent-proposed `new` modules onto the `current` set. Current modules
/// are kept verbatim (preserving ids); a new module is appended only when its
/// name and its `root_subdir` are both distinct from every current module, so
/// the agent can't duplicate what already exists.
fn merge(current: &[Module], new: Vec<Module>) -> Vec<Module> {
    let names: std::collections::HashSet<String> =
        current.iter().map(|m| m.name.to_lowercase()).collect();
    let subdirs: std::collections::HashSet<PathBuf> =
        current.iter().map(|m| m.root_subdir.clone()).collect();

    let mut out = current.to_vec();
    for m in new {
        if names.contains(&m.name.to_lowercase()) || subdirs.contains(&m.root_subdir) {
            continue;
        }
        out.push(m);
    }
    out
}

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
    if lines.len() > MAX_LISTING_LINES {
        let omitted = lines.len() - MAX_LISTING_LINES;
        lines.truncate(MAX_LISTING_LINES);
        lines.push(format!("… (listing truncated — {omitted} more entries omitted)"));
    }
    if lines.is_empty() {
        "(empty)".to_string()
    } else {
        lines.join("\n")
    }
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth == 0 || out.len() > MAX_LISTING_LINES {
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
        if out.len() > MAX_LISTING_LINES {
            return;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let (file_count, subdir_count) = child_counts(&path);
        // Collapse big container dirs (e.g. packages/* in a monorepo) instead of
        // expanding every child, keeping the high-signal fact without the bulk.
        if subdir_count > MAX_CHILDREN_LISTED {
            out.push(format!(
                "{} ({file_count} files, {subdir_count} subdirs — collapsed)",
                rel.display()
            ));
            continue;
        }
        out.push(format!("{} ({file_count} files)", rel.display()));
        walk(root, &path, depth - 1, out);
    }
}

/// Returns (immediate file count, immediate non-noise/non-hidden subdir count).
fn child_counts(dir: &Path) -> (usize, usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    let mut files = 0;
    let mut subdirs = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files += 1;
        } else if path.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || is_noise(&name) {
                continue;
            }
            subdirs += 1;
        }
    }
    (files, subdirs)
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

    fn module(name: &str, subdir: &str) -> Module {
        Module {
            id: netra_core::ids::ModuleId::new(),
            name: name.into(),
            description: "d".into(),
            root_subdir: PathBuf::from(subdir),
        }
    }

    #[test]
    fn merge_keeps_current_and_appends_only_distinct_new() {
        let current = vec![module("root", ""), module("backend", "server")];
        let current_ids: Vec<_> = current.iter().map(|m| m.id).collect();
        let new = vec![
            module("backend", "server2"), // dup name → drop
            module("ui", "server"),       // dup subdir → drop
            module("frontend", "web"),    // distinct → keep
        ];
        let merged = merge(&current, new);
        let names: Vec<_> = merged.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["root", "backend", "frontend"]);
        // Current ids preserved so the UI diff aligns them as "kept".
        assert_eq!(merged[0].id, current_ids[0]);
        assert_eq!(merged[1].id, current_ids[1]);
    }

    #[test]
    fn format_existing_renders_root_and_named() {
        let mods = vec![module("root", ""), module("backend", "server")];
        let s = format_existing(&mods);
        assert!(s.contains("whole repo"), "got: {s}");
        assert!(s.contains("backend [server]"), "got: {s}");
    }

    #[test]
    fn listing_collapses_large_container() {
        let dir = tempdir().unwrap();
        let packages = dir.path().join("packages");
        std::fs::create_dir(&packages).unwrap();
        for i in 0..(MAX_CHILDREN_LISTED + 5) {
            std::fs::create_dir(packages.join(format!("pkg{i}"))).unwrap();
        }
        let listing = directory_listing(dir.path(), 2);
        assert!(listing.contains("collapsed"), "got: {listing}");
        // No individual child should be expanded.
        assert!(!listing.contains("packages/pkg0"), "got: {listing}");
    }

    #[tokio::test]
    async fn structured_inference_validates_and_merges() {
        use netra_engine::MockPlanner;
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("server")).unwrap();
        let mut project = Project::new("p", dir.path());
        project.modules = vec![module("root", "")];

        let planner = MockPlanner::new(serde_json::json!({
            "modules": [
                { "name": "backend", "description": "api", "root_subdir": "server" },
                { "name": "ghost", "description": "x", "root_subdir": "missing" }
            ]
        }));
        let out = infer_modules_structured(&project, &planner).await.unwrap();
        let names: Vec<_> = out.iter().map(|m| m.name.as_str()).collect();
        // root kept, backend added (subdir exists), ghost dropped (subdir missing).
        assert_eq!(names, vec!["root", "backend"]);
        assert_eq!(planner.call_count(), 1);
    }

    #[test]
    fn modules_json_roundtrips_and_validates() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("server")).unwrap();
        let mods = vec![module("backend", "server")];
        let json = modules_to_json(&mods);
        assert!(json.contains("netra.modules/v1"));
        assert!(!json.contains("\"id\""), "ids must be omitted from export");

        let parsed = modules_from_json(&json, dir.path()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "backend");
        assert_eq!(parsed[0].root_subdir, PathBuf::from("server"));
    }

    #[test]
    fn modules_from_json_rejects_bad_format() {
        let dir = tempdir().unwrap();
        let err = modules_from_json(r#"{"format":"nope","modules":[]}"#, dir.path())
            .unwrap_err();
        assert!(format!("{err}").contains("unsupported format"));
    }

    #[test]
    fn listing_caps_total_lines() {
        let dir = tempdir().unwrap();
        for i in 0..(MAX_LISTING_LINES + 50) {
            std::fs::create_dir(dir.path().join(format!("d{i:04}"))).unwrap();
        }
        let listing = directory_listing(dir.path(), 2);
        let line_count = listing.lines().count();
        assert!(line_count <= MAX_LISTING_LINES + 1, "got {line_count} lines");
        assert!(listing.contains("listing truncated"), "got: {listing}");
    }
}
