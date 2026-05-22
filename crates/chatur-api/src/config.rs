//! Application configuration, loaded from a TOML file.
//!
//! Every field has a default, so a missing config file (or a partial one) is
//! valid: see [`ChaturConfig::load_or_default`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Top-level Mini ChatUR configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ChaturConfig {
    /// Path to the `pi` executable (looked up on `PATH` if just a name).
    pub pi_binary: PathBuf,
    /// Directory for the SQLite database and other runtime state.
    pub data_dir: PathBuf,
    /// Directory for per-job log files.
    pub log_dir: PathBuf,
    /// Concurrency limits for the scheduler.
    pub concurrency: ConcurrencyConfig,
    /// Default model used when neither job nor project specifies one.
    pub default_model: Option<ModelConfig>,
}

impl Default for ChaturConfig {
    fn default() -> Self {
        Self {
            pi_binary: PathBuf::from("pi"),
            data_dir: PathBuf::from(".chatur/data"),
            log_dir: PathBuf::from(".chatur/logs"),
            concurrency: ConcurrencyConfig::default(),
            default_model: None,
        }
    }
}

impl ChaturConfig {
    /// Loads configuration from a TOML file at `path`.
    ///
    /// # Errors
    /// Returns [`ConfigError::Read`] if the file cannot be read, or
    /// [`ConfigError::Parse`] if its contents are not valid config TOML.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|source| ConfigError::Read(path.to_path_buf(), source))?;
        let config = toml::from_str(&text)?;
        Ok(config)
    }

    /// Loads configuration from `path`, falling back to defaults when the file
    /// does not exist.
    ///
    /// # Errors
    /// Returns an error only if the file exists but cannot be read or parsed.
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }
}

/// Scheduler concurrency limits.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct ConcurrencyConfig {
    /// Maximum agent jobs running at once across all projects.
    pub global_max: usize,
    /// Maximum agent jobs running at once for any single project.
    pub per_project_max: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            global_max: 4,
            per_project_max: 2,
        }
    }
}

/// A provider + model selection in the config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Provider name, e.g. `anthropic`.
    pub provider: String,
    /// Model id or pattern.
    pub model: String,
}

/// Errors raised while loading [`ChaturConfig`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The config file could not be read.
    #[error("failed to read config file {0}: {1}")]
    Read(PathBuf, #[source] std::io::Error),
    /// The config file was not valid TOML for this schema.
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let config = ChaturConfig::default();
        assert_eq!(config.pi_binary, PathBuf::from("pi"));
        assert_eq!(config.concurrency.global_max, 4);
        assert_eq!(config.concurrency.per_project_max, 2);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let config = ChaturConfig::load_or_default("/nonexistent/chatur.toml")
            .expect("missing file must fall back to defaults");
        assert_eq!(config.concurrency.global_max, 4);
    }

    #[test]
    fn partial_toml_merges_with_defaults() {
        let text = r#"
            pi_binary = "/usr/local/bin/pi"
            [concurrency]
            global_max = 8
        "#;
        let config: ChaturConfig = toml::from_str(text).expect("valid partial config");
        assert_eq!(config.pi_binary, PathBuf::from("/usr/local/bin/pi"));
        assert_eq!(config.concurrency.global_max, 8);
        // Unspecified fields keep their defaults.
        assert_eq!(config.concurrency.per_project_max, 2);
        assert_eq!(config.log_dir, PathBuf::from(".chatur/logs"));
    }

    #[test]
    fn invalid_toml_is_an_error() {
        assert!(toml::from_str::<ChaturConfig>("concurrency = \"not a table\"").is_err());
    }
}
