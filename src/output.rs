use serde::Serialize;
use std::io::{self, Write};

/// Hook execution output containing exit code and optional messages.
#[derive(Debug)]
pub struct Output {
    /// Exit code (0 for success, 2 for block).
    pub exit_code: i32,
    /// Standard output content.
    pub stdout: Option<String>,
    /// Standard error content.
    pub stderr: Option<String>,
}

/// Creates an output that blocks the tool execution with an optional message.
pub fn block_output(message: Option<&str>) -> Output {
    Output {
        exit_code: 2,
        stdout: None,
        stderr: message.map(std::string::ToString::to_string),
    }
}

/// Creates an output indicating no rule matched (allows the tool to proceed).
pub fn no_match_output() -> Output {
    Output {
        exit_code: 0,
        stdout: None,
        stderr: None,
    }
}

#[derive(Serialize)]
struct HookSpecificOutput {
    #[serde(rename = "hookEventName")]
    hook_event_name: String,
    #[serde(rename = "permissionDecision")]
    permission_decision: String,
    #[serde(rename = "updatedInput")]
    updated_input: serde_json::Value,
}

#[derive(Serialize)]
struct TransformOutput {
    #[serde(rename = "hookSpecificOutput")]
    hook_specific_output: HookSpecificOutput,
}

/// Creates an output that transforms the tool input with an updated command.
pub fn transform_output(event_name: &str, updated_command: &str) -> Output {
    let output = TransformOutput {
        hook_specific_output: HookSpecificOutput {
            hook_event_name: event_name.to_string(),
            permission_decision: "allow".to_string(),
            updated_input: serde_json::json!({ "command": updated_command }),
        },
    };

    let stdout = match serde_json::to_string(&output) {
        Ok(json) => Some(json),
        Err(e) => {
            eprintln!("Warning: failed to serialize transform output: {e}");
            None
        }
    };

    Output {
        exit_code: 0,
        stdout,
        stderr: None,
    }
}

/// Writes the output to stdout and stderr streams.
pub fn emit(output: &Output) {
    if let Some(ref stdout_content) = output.stdout {
        if let Err(e) = io::stdout().write_all(stdout_content.as_bytes()) {
            eprintln!("Warning: failed to write to stdout: {e}");
        }
        if let Err(e) = io::stdout().flush() {
            eprintln!("Warning: failed to flush stdout: {e}");
        }
    }

    if let Some(ref stderr_content) = output.stderr {
        if let Err(e) = io::stderr().write_all(stderr_content.as_bytes()) {
            eprintln!("Warning: failed to write to stderr: {e}");
        }
        if let Err(e) = io::stderr().write_all(b"\n") {
            eprintln!("Warning: failed to write newline to stderr: {e}");
        }
        if let Err(e) = io::stderr().flush() {
            eprintln!("Warning: failed to flush stderr: {e}");
        }
    }
}
