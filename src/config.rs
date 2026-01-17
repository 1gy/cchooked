use crate::error::{CchookedError, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Default path for the hooks rules configuration file.
pub const DEFAULT_CONFIG_PATH: &str = ".claude/hooks-rules.toml";

/// Root configuration containing all hook rules.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Map of rule names to their configurations.
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,
}

fn default_on_error() -> String {
    "ignore".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

/// Configuration for a single hook rule.
#[derive(Debug, Deserialize)]
pub struct RuleConfig {
    /// Event type (`PreToolUse` or `PostToolUse`).
    pub event: String,
    /// Regex pattern to match tool names.
    pub matcher: String,
    /// Action to perform (block, transform, run, or log).
    pub action: String,
    /// Priority for rule ordering (higher values are evaluated first).
    #[serde(default)]
    pub priority: i32,
    /// Optional message for block actions.
    pub message: Option<String>,
    /// Optional conditional filters.
    #[serde(default)]
    pub when: Option<WhenConfig>,
    /// Optional transform configuration.
    #[serde(default)]
    pub transform: Option<TransformConfig>,
    /// Command template for run actions.
    pub command: Option<String>,
    /// Behavior when command fails (ignore or fail).
    #[serde(default = "default_on_error")]
    pub on_error: String,
    /// File path for log actions.
    pub log_file: Option<String>,
    /// Log format (text or json).
    #[serde(default = "default_log_format")]
    pub log_format: String,
}

/// Configuration for command transformation.
#[derive(Debug, Default, Deserialize)]
pub struct TransformConfig {
    /// Regex pattern and replacement pair for command transformation.
    pub command: Option<[String; 2]>,
}

/// Conditional filter configuration for rule matching.
#[derive(Debug, Default, Deserialize)]
pub struct WhenConfig {
    /// Regex patterns to match against the command.
    pub command: Option<StringOrVec>,
    /// Regex patterns to match against the file path.
    pub file_path: Option<StringOrVec>,
    /// Branch names to match against.
    pub branch: Option<StringOrVec>,
}

/// A flexible type that accepts either a single string or an array of strings.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrVec {
    /// A single string value.
    Single(String),
    /// Multiple string values.
    Multiple(Vec<String>),
}

impl StringOrVec {
    /// Converts the value to a vector of strings.
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StringOrVec::Single(s) => vec![s.clone()],
            StringOrVec::Multiple(v) => v.clone(),
        }
    }
}

/// Loads the configuration from a file.
///
/// If no path is provided, uses the default configuration path.
pub fn load_config(path: Option<&str>) -> Result<Config> {
    let config_path = path.unwrap_or(DEFAULT_CONFIG_PATH);
    let path = Path::new(config_path);

    if !path.exists() {
        return Err(CchookedError::ConfigNotFound(config_path.to_string()));
    }

    let content = fs::read_to_string(path)?;

    let config: Config = toml::from_str(&content).map_err(|e| CchookedError::ConfigParseError {
        path: config_path.to_string(),
        detail: e.to_string(),
    })?;

    Ok(config)
}
