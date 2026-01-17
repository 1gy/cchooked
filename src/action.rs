use crate::context::Context;
use crate::output::{self, Output};
use crate::rule::{ActionType, EventType, LogFormat, MatchResult, OnErrorBehavior};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

/// Resolves the working directory from the template or falls back to the context's file_dir.
fn resolve_working_dir(working_dir: Option<&String>, context: &Context) -> Option<String> {
    let dir = match working_dir {
        Some(template) => {
            let expanded = context.expand(template);
            if expanded.is_empty() {
                context.file_dir.clone()
            } else if std::path::Path::new(&expanded).is_absolute() {
                expanded
            } else {
                format!("{}/{}", context.workspace_root, expanded)
            }
        }
        None => context.file_dir.clone(),
    };

    if dir.is_empty() { None } else { Some(dir) }
}

/// Executes the action based on the match result.
///
/// Processes the matched rule's action (Block, Transform, Run, or Log) and returns the appropriate output.
pub fn execute_action(match_result: &MatchResult, context: &Context, event: &EventType) -> Output {
    match match_result.action {
        ActionType::Block => {
            let message = match_result.message.as_ref().map(|m| context.expand(m));
            output::block_output(message.as_deref())
        }
        ActionType::Transform => {
            if let Some(ref transform) = match_result.transform
                && let (Some(pattern), Some(replacement)) =
                    (&transform.command_pattern, &transform.command_replacement)
            {
                let transformed = pattern.replace(&context.command, replacement.as_str());
                return output::transform_output(event.as_str(), &transformed);
            }
            output::no_match_output()
        }
        ActionType::Run => {
            if let Some(ref cmd_template) = match_result.run_command {
                let cmd = context.expand(cmd_template);

                let working_dir = resolve_working_dir(match_result.working_dir.as_ref(), context);

                if let Some(ref dir) = working_dir
                    && !std::path::Path::new(dir).exists()
                {
                    return match match_result.on_error {
                        OnErrorBehavior::Ignore => output::no_match_output(),
                        OnErrorBehavior::Fail => output::block_output(Some(&format!(
                            "Working directory does not exist: {}",
                            dir
                        ))),
                    };
                }

                let mut command = Command::new("sh");
                command.args(["-c", &cmd]);

                if let Some(ref dir) = working_dir {
                    command.current_dir(dir);
                }

                let result = command.output();

                match result {
                    Ok(output_result) if output_result.status.success() => {
                        output::no_match_output()
                    }
                    Ok(output_result) => {
                        let stderr = String::from_utf8_lossy(&output_result.stderr);
                        match match_result.on_error {
                            OnErrorBehavior::Ignore => output::no_match_output(),
                            OnErrorBehavior::Fail => {
                                output::block_output(Some(&format!("Command failed: {stderr}")))
                            }
                        }
                    }
                    Err(e) => match match_result.on_error {
                        OnErrorBehavior::Ignore => output::no_match_output(),
                        OnErrorBehavior::Fail => {
                            output::block_output(Some(&format!("Failed to run command: {e}")))
                        }
                    },
                }
            } else {
                output::no_match_output()
            }
        }
        ActionType::Log => {
            let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

            let log_entry = match match_result.log_format {
                LogFormat::Text => {
                    let log_content = if context.command.is_empty() {
                        &context.file_path
                    } else {
                        &context.command
                    };
                    format!(
                        "[{timestamp}] {} {}: {log_content}",
                        event.as_str(),
                        &context.tool_name,
                    )
                }
                LogFormat::Json => {
                    let obj = serde_json::json!({
                        "timestamp": timestamp,
                        "event": event.as_str(),
                        "tool": &context.tool_name,
                        "command": &context.command,
                        "file_path": &context.file_path,
                    });
                    serde_json::to_string(&obj).unwrap_or_default()
                }
            };

            if let Some(ref file_path) = match_result.log_file {
                let expanded_path = if file_path.starts_with('~') {
                    if let Ok(home) = std::env::var("HOME") {
                        file_path.replacen('~', &home, 1)
                    } else {
                        file_path.clone()
                    }
                } else {
                    file_path.clone()
                };

                if let Some(parent) = std::path::Path::new(&expanded_path).parent()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    eprintln!("Warning: failed to create log directory: {e}");
                }

                match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&expanded_path)
                {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{log_entry}") {
                            eprintln!("Warning: failed to write log entry: {e}");
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: failed to open log file '{}': {e}", expanded_path);
                    }
                }
            }

            output::no_match_output()
        }
    }
}
