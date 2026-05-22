//! How to launch a `pi` process.

use std::path::PathBuf;

use chatur_core::model::{ModelRef, ToolPolicy};

/// The parameters needed to spawn one `pi --mode rpc` process.
#[derive(Debug, Clone)]
pub struct AgentSpec {
    /// Path to (or name of) the `pi` executable.
    pub binary: PathBuf,
    /// Working directory — the code project the agent operates on.
    pub cwd: PathBuf,
    /// Provider + model override; `None` uses `pi`'s configured default.
    pub model: Option<ModelRef>,
    /// Which tools the agent may use.
    pub tool_policy: ToolPolicy,
    /// Existing `pi` session id to resume; `None` starts fresh.
    pub session: Option<String>,
    /// Additional raw arguments appended verbatim to the `pi` command line.
    pub extra_args: Vec<String>,
}

impl AgentSpec {
    /// Creates a spec with a read-only tool policy and no model override.
    #[must_use]
    pub fn new(binary: impl Into<PathBuf>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            cwd: cwd.into(),
            model: None,
            tool_policy: ToolPolicy::ReadOnly,
            session: None,
            extra_args: Vec::new(),
        }
    }

    /// Sets the provider + model override.
    #[must_use]
    pub fn with_model(mut self, model: ModelRef) -> Self {
        self.model = Some(model);
        self
    }

    /// Sets the tool policy.
    #[must_use]
    pub fn with_tool_policy(mut self, policy: ToolPolicy) -> Self {
        self.tool_policy = policy;
        self
    }

    /// Sets a `pi` session id to resume.
    #[must_use]
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }

    /// Builds the full `pi` argument vector for this spec.
    pub(crate) fn build_args(&self) -> Vec<String> {
        let mut args = vec!["--mode".to_string(), "rpc".to_string()];

        if let Some(model) = &self.model {
            args.push("--provider".to_string());
            args.push(model.provider.clone());
            args.push("--model".to_string());
            args.push(model.model.clone());
        }

        match &self.tool_policy {
            ToolPolicy::ReadOnly => args.push("--no-tools".to_string()),
            ToolPolicy::Allowlist(tools) if tools.is_empty() => {
                args.push("--no-tools".to_string());
            }
            ToolPolicy::Allowlist(tools) => {
                args.push("--tools".to_string());
                args.push(tools.join(","));
            }
            ToolPolicy::Full => {}
        }

        if let Some(session) = &self.session {
            args.push("--session".to_string());
            args.push(session.clone());
        }

        args.extend(self.extra_args.iter().cloned());
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_policy_disables_tools() {
        let spec = AgentSpec::new("pi", "/tmp");
        assert!(spec.build_args().contains(&"--no-tools".to_string()));
    }

    #[test]
    fn model_override_emits_provider_and_model() {
        let spec = AgentSpec::new("pi", "/tmp").with_model(ModelRef {
            provider: "llamacpp".to_string(),
            model: "qwen3.6-35b-a3b".to_string(),
        });
        let args = spec.build_args();
        assert!(args.windows(2).any(|w| w == ["--provider", "llamacpp"]));
        assert!(args.windows(2).any(|w| w == ["--model", "qwen3.6-35b-a3b"]));
    }

    #[test]
    fn allowlist_policy_joins_tool_names() {
        let spec = AgentSpec::new("pi", "/tmp")
            .with_tool_policy(ToolPolicy::Allowlist(vec!["read".into(), "grep".into()]));
        let args = spec.build_args();
        assert!(args.windows(2).any(|w| w == ["--tools", "read,grep"]));
    }
}
