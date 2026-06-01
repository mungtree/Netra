//! Reusable, versioned prompt templates.

use serde::{Deserialize, Serialize};

use crate::ids::TemplateId;

/// A named prompt body with substitutable `{{variable}}` placeholders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Stable identifier.
    pub id: TemplateId,
    /// Human-readable name.
    pub name: String,
    /// Template body; placeholders use `{{name}}` syntax.
    pub body: String,
    /// Declared variable names expected by [`body`](Self::body).
    pub variables: Vec<String>,
    /// Monotonic version, bumped on every edit.
    pub version: u32,
}

impl PromptTemplate {
    /// Creates a version-1 template with a fresh id.
    #[must_use]
    pub fn new(name: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id: TemplateId::new(),
            name: name.into(),
            body: body.into(),
            variables: Vec::new(),
            version: 1,
        }
    }
}
