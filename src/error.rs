use std::fmt;
use std::io;

/// Error types for the cchooked hook engine.
#[derive(Debug)]
pub enum CchookedError {
    /// Configuration file was not found at the specified path.
    ConfigNotFound(String),
    /// Failed to parse the configuration file.
    ConfigParseError { path: String, detail: String },
    /// Failed to parse the input JSON.
    InputParseError(String),
    /// Invalid regex pattern in a rule.
    RegexError {
        rule_name: String,
        pattern: String,
        detail: String,
    },
    /// Invalid event type specified.
    InvalidEventType {
        value: String,
        valid: Vec<&'static str>,
    },
    /// Invalid action type specified.
    InvalidActionType {
        value: String,
        valid: Vec<&'static str>,
    },
    /// Log action specified without a log file path.
    LogFileMissing { rule_name: String },
    /// IO error occurred.
    IoError(io::Error),
}

impl fmt::Display for CchookedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CchookedError::ConfigNotFound(path) => {
                write!(f, "Config file not found: {path}")
            }
            CchookedError::ConfigParseError { path, detail } => {
                write!(f, "Failed to parse config file '{path}':\n  {detail}")
            }
            CchookedError::InputParseError(detail) => {
                write!(f, "Failed to parse input JSON: {detail}")
            }
            CchookedError::RegexError {
                rule_name,
                pattern,
                detail,
            } => {
                write!(
                    f,
                    "Invalid regex in rule '{rule_name}': pattern '{pattern}' - {detail}"
                )
            }
            CchookedError::InvalidEventType { value, valid } => {
                write!(
                    f,
                    "Invalid event type '{value}'. Valid values: {}",
                    valid.join(", ")
                )
            }
            CchookedError::InvalidActionType { value, valid } => {
                write!(
                    f,
                    "Invalid action type '{value}'. Valid values: {}",
                    valid.join(", ")
                )
            }
            CchookedError::LogFileMissing { rule_name } => {
                write!(
                    f,
                    "Rule '{rule_name}' uses log action but log_file is not specified"
                )
            }
            CchookedError::IoError(e) => {
                write!(f, "IO error: {e}")
            }
        }
    }
}

impl std::error::Error for CchookedError {}

impl From<io::Error> for CchookedError {
    fn from(err: io::Error) -> Self {
        CchookedError::IoError(err)
    }
}

impl From<serde_json::Error> for CchookedError {
    fn from(err: serde_json::Error) -> Self {
        CchookedError::InputParseError(err.to_string())
    }
}

/// A Result type alias using `CchookedError` as the error type.
pub type Result<T> = std::result::Result<T, CchookedError>;
