//! Structured reviewer output: a typed list of findings.
//!
//! The `structured_reviewer` aggregation strategy asks the model to produce a
//! [`FindingsReport`] (summary + array of [`Finding`]s) so the front-end can
//! render bugs, ideas, fixes, and other items with severity and metadata
//! instead of free-form prose.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The category of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    Bug,
    Vulnerability,
    Idea,
    Change,
    Fix,
    Suggestion,
    Other,
}

/// How urgent or impactful a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// One reviewer-produced item: bug, idea, fix, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    /// `path/to/file.rs:42` when known.
    #[serde(default)]
    pub location: Option<String>,
    /// Short suggested remediation when applicable.
    #[serde(default)]
    pub suggested_fix: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// The consolidated reviewer output: a brief summary plus a typed finding list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsReport {
    pub summary: String,
    pub findings: Vec<Finding>,
}

/// JSON Schema describing [`FindingsReport`].
///
/// Hand-rolled (no `schemars` dependency in this crate) and passed to the
/// reviewer agent so its output conforms to [`FindingsReport`].
#[must_use]
pub fn findings_report_schema() -> Value {
    let kinds = json!([
        "bug", "vulnerability", "idea", "change", "fix", "suggestion", "other"
    ]);
    let severities = json!(["critical", "high", "medium", "low", "info"]);

    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "FindingsReport",
        "type": "object",
        "required": ["summary", "findings"],
        "properties": {
            "summary": { "type": "string" },
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["kind", "severity", "title", "description"],
                    "properties": {
                        "kind": { "enum": kinds },
                        "severity": { "enum": severities },
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "location": { "type": ["string", "null"] },
                        "suggested_fix": { "type": ["string", "null"] },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let report = FindingsReport {
            summary: "ok".into(),
            findings: vec![Finding {
                kind: FindingKind::Bug,
                severity: Severity::High,
                title: "off-by-one".into(),
                description: "loop bounds".into(),
                location: Some("src/x.rs:10".into()),
                suggested_fix: Some("use ..=".into()),
                tags: vec!["loop".into()],
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: FindingsReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.findings[0].title, "off-by-one");
    }

    #[test]
    fn schema_has_required_top_level_fields() {
        let schema = findings_report_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "summary"));
        assert!(required.iter().any(|v| v == "findings"));
    }
}
