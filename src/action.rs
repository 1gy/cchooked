use crate::context::Context;
use crate::output::{self, Output};
use crate::rule::{ActionType, EventType, LogFormat, MatchResult, OnErrorBehavior};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

/// Resolves the working directory from the template or falls back to the context's file_dir.
fn resolve_working_dir(working_dir: Option<&String>, context: &Context) -> Option<String> {
    match working_dir {
        Some(template) => {
            let expanded = context.expand(template);
            if expanded.is_empty() {
                if context.file_dir.is_empty() {
                    None
                } else {
                    Some(context.file_dir.clone())
                }
            } else if std::path::Path::new(&expanded).is_absolute() {
                Some(expanded)
            } else {
                let base = if context.workspace_root.is_empty() {
                    std::path::PathBuf::from(&context.file_dir)
                } else {
                    std::path::PathBuf::from(&context.workspace_root)
                };
                Some(base.join(&expanded).to_string_lossy().into_owned())
            }
        }
        None => {
            if context.file_dir.is_empty() {
                None
            } else {
                Some(context.file_dir.clone())
            }
        }
    }
}

/// Executes the action based on the match result.
///
/// Processes the matched rule's action (Block, Run, or Log) and returns the appropriate output.
pub fn execute_action(match_result: &MatchResult, context: &Context, event: &EventType) -> Output {
    match match_result.action {
        ActionType::Block => {
            let message = match_result.message.as_ref().map(|m| context.expand(m));
            output::block_output(message.as_deref())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context(
        file_dir: &str,
        workspace_root: &str,
        command: &str,
        file_path: &str,
    ) -> Context {
        Context {
            command: command.to_string(),
            file_path: file_path.to_string(),
            file_dir: file_dir.to_string(),
            tool_name: "Bash".to_string(),
            branch: "main".to_string(),
            workspace_root: workspace_root.to_string(),
        }
    }

    #[test]
    fn test_resolve_working_dir_none_with_file_dir() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let result = resolve_working_dir(None, &ctx);
        assert_eq!(result, Some("/home/user/project/src".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_none_with_empty_file_dir() {
        let ctx = make_context("", "/home/user/project", "", "");
        let result = resolve_working_dir(None, &ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_working_dir_empty_template_with_file_dir() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = String::new();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/src".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_empty_template_with_empty_file_dir() {
        let ctx = make_context("", "/home/user/project", "", "");
        let template = String::new();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_working_dir_absolute_path() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "/absolute/path".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/absolute/path".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_relative_path_with_workspace_root() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "subdir".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/subdir".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_relative_path_with_empty_workspace_root() {
        let ctx = make_context("/home/user/project/src", "", "", "");
        let template = "subdir".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/src/subdir".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_relative_path_with_both_empty() {
        let ctx = make_context("", "", "", "");
        let template = "subdir".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("subdir".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_template_expansion() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "${file_dir}/subdir".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/src/subdir".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_template_expands_to_empty() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        // ${command} is empty, so template expands to empty string
        let template = "${command}".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/src".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_template_expands_to_empty_with_empty_file_dir() {
        let ctx = make_context("", "/home/user/project", "", "");
        let template = "${command}".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_working_dir_nested_relative_path() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "foo/bar/baz".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/foo/bar/baz".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_dot_relative_path() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "./subdir".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/./subdir".to_string()));
    }

    #[test]
    fn test_resolve_working_dir_parent_relative_path() {
        let ctx = make_context("/home/user/project/src", "/home/user/project", "", "");
        let template = "../other".to_string();
        let result = resolve_working_dir(Some(&template), &ctx);
        assert_eq!(result, Some("/home/user/project/../other".to_string()));
    }
}
