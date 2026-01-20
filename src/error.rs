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

impl CchookedError {
    /// Returns the appropriate exit code for this error.
    ///
    /// Exit codes follow the Claude Code hooks protocol:
    /// - 0: Non-blocking (config not found, allows tool to proceed)
    /// - 2: Blocking (configuration errors that must be fixed by the user)
    ///
    /// Note: Exit code 1 is intentionally not used because Claude Code treats it
    /// as a "non-blocking error" and continues execution, which would silently
    /// disable all protection rules.
    pub fn exit_code(&self) -> i32 {
        match self {
            // Config not found is not an error - hooks are optional
            CchookedError::ConfigNotFound(_) => 0,
            // All other errors are configuration/setup errors that should block
            CchookedError::ConfigParseError { .. }
            | CchookedError::InputParseError(_)
            | CchookedError::RegexError { .. }
            | CchookedError::InvalidEventType { .. }
            | CchookedError::InvalidActionType { .. }
            | CchookedError::LogFileMissing { .. }
            | CchookedError::IoError(_) => 2,
        }
    }

    /// Returns true if this error should be treated as a warning rather than an error.
    ///
    /// Warning-level errors allow the tool to proceed (exit code 0).
    pub fn is_warning(&self) -> bool {
        matches!(self, CchookedError::ConfigNotFound(_))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_code_config_not_found() {
        let err = CchookedError::ConfigNotFound("/path/to/config".to_string());
        assert_eq!(err.exit_code(), 0);
        assert!(err.is_warning());
    }

    #[test]
    fn test_exit_code_config_parse_error() {
        let err = CchookedError::ConfigParseError {
            path: "/path".to_string(),
            detail: "syntax error".to_string(),
        };
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_input_parse_error() {
        let err = CchookedError::InputParseError("invalid json".to_string());
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_regex_error() {
        let err = CchookedError::RegexError {
            rule_name: "test".to_string(),
            pattern: "[invalid".to_string(),
            detail: "unclosed bracket".to_string(),
        };
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_invalid_event_type() {
        let err = CchookedError::InvalidEventType {
            value: "BadEvent".to_string(),
            valid: vec!["PreToolUse", "PostToolUse"],
        };
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_invalid_action_type() {
        let err = CchookedError::InvalidActionType {
            value: "bad_action".to_string(),
            valid: vec!["block", "run", "log"],
        };
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_log_file_missing() {
        let err = CchookedError::LogFileMissing {
            rule_name: "test".to_string(),
        };
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }

    #[test]
    fn test_exit_code_io_error() {
        let err = CchookedError::IoError(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        assert_eq!(err.exit_code(), 2);
        assert!(!err.is_warning());
    }
}
