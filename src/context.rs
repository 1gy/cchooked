use crate::rule::HookInput;
use std::process::Command;

/// Execution context containing extracted input values and environment information.
#[derive(Debug)]
pub struct Context {
    /// Command string from tool input.
    pub command: String,
    /// File path from tool input.
    pub file_path: String,
    /// Parent directory of file_path.
    pub file_dir: String,
    /// Name of the tool being invoked.
    pub tool_name: String,
    /// Current git branch name.
    pub branch: String,
    /// Current working directory of cchooked.
    pub workspace_root: String,
}

impl Context {
    /// Creates a new context from hook input.
    ///
    /// Extracts command, file path, tool name, and detects the current git branch.
    pub fn from_input(input: &HookInput) -> Self {
        let file_path = input.tool_input.file_path.clone().unwrap_or_default();
        let file_dir = if file_path.is_empty() {
            String::new()
        } else {
            std::path::Path::new(&file_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        };
        let workspace_root = std::env::var("CLAUDE_PROJECT_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default()
            });

        Self {
            command: input.tool_input.command.clone().unwrap_or_default(),
            file_path,
            file_dir,
            tool_name: input.tool_name.clone(),
            branch: get_current_branch().unwrap_or_default(),
            workspace_root,
        }
    }

    /// Expands template variables in a string.
    ///
    /// Replaces `${command}`, `${file_path}`, `${file_dir}`, `${tool_name}`, `${branch}`, and `${workspace_root}` with their values.
    pub fn expand(&self, template: &str) -> String {
        template
            .replace("${command}", &self.command)
            .replace("${file_path}", &self.file_path)
            .replace("${file_dir}", &self.file_dir)
            .replace("${tool_name}", &self.tool_name)
            .replace("${branch}", &self.branch)
            .replace("${workspace_root}", &self.workspace_root)
    }
}

fn get_current_branch() -> Option<String> {
    // Allow overriding via environment variable for testing
    if let Ok(branch) = std::env::var("CCHOOKED_BRANCH") {
        return Some(branch);
    }

    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::ToolInput;

    #[test]
    fn test_context_from_input() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("npm install".to_string()),
                file_path: None,
            },
        };

        let ctx = Context::from_input(&input);

        assert_eq!(ctx.tool_name, "Bash");
        assert_eq!(ctx.command, "npm install");
        assert_eq!(ctx.file_path, "");
        assert_eq!(ctx.file_dir, "");
        assert!(!ctx.workspace_root.is_empty());
    }

    #[test]
    fn test_context_from_input_with_file_path() {
        let input = HookInput {
            tool_name: "Read".to_string(),
            tool_input: ToolInput {
                command: None,
                file_path: Some("/home/user/project/src/main.rs".to_string()),
            },
        };

        let ctx = Context::from_input(&input);

        assert_eq!(ctx.tool_name, "Read");
        assert_eq!(ctx.file_path, "/home/user/project/src/main.rs");
        assert_eq!(ctx.file_dir, "/home/user/project/src");
        assert!(!ctx.workspace_root.is_empty());
    }

    #[test]
    fn test_expand_variables() {
        let ctx = Context {
            command: "npm test".to_string(),
            file_path: "/src/main.rs".to_string(),
            file_dir: "/src".to_string(),
            tool_name: "Bash".to_string(),
            branch: "main".to_string(),
            workspace_root: "/home/user/project".to_string(),
        };

        let result = ctx.expand("Running ${command} on ${branch}");
        assert_eq!(result, "Running npm test on main");

        let result = ctx.expand("File: ${file_path}, Tool: ${tool_name}");
        assert_eq!(result, "File: /src/main.rs, Tool: Bash");

        let result = ctx.expand("Dir: ${file_dir}, Root: ${workspace_root}");
        assert_eq!(result, "Dir: /src, Root: /home/user/project");
    }

    #[test]
    fn test_expand_no_variables() {
        let ctx = Context {
            command: "test".to_string(),
            file_path: "".to_string(),
            file_dir: "".to_string(),
            tool_name: "Bash".to_string(),
            branch: "main".to_string(),
            workspace_root: "/home/user/project".to_string(),
        };

        let result = ctx.expand("No variables here");
        assert_eq!(result, "No variables here");
    }

    #[test]
    fn test_file_dir_from_root_file() {
        let ctx = Context {
            command: "".to_string(),
            file_path: "/main.rs".to_string(),
            file_dir: "/".to_string(),
            tool_name: "Read".to_string(),
            branch: "main".to_string(),
            workspace_root: "/home/user/project".to_string(),
        };

        let result = ctx.expand("${file_dir}");
        assert_eq!(result, "/");
    }

    #[test]
    fn test_workspace_root_uses_claude_project_dir() {
        // Save the original value
        let original = std::env::var("CLAUDE_PROJECT_DIR").ok();

        // SAFETY: This test is run with --test-threads=1 to avoid race conditions
        unsafe {
            std::env::set_var("CLAUDE_PROJECT_DIR", "/custom/project/dir");
        }

        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("test".to_string()),
                file_path: None,
            },
        };

        let ctx = Context::from_input(&input);

        assert_eq!(ctx.workspace_root, "/custom/project/dir");

        // Restore the original value
        // SAFETY: This test is run with --test-threads=1 to avoid race conditions
        unsafe {
            match original {
                Some(val) => std::env::set_var("CLAUDE_PROJECT_DIR", val),
                None => std::env::remove_var("CLAUDE_PROJECT_DIR"),
            }
        }
    }

    #[test]
    fn test_workspace_root_fallback_when_claude_project_dir_empty() {
        // Save the original value
        let original = std::env::var("CLAUDE_PROJECT_DIR").ok();

        // SAFETY: This test is run with --test-threads=1 to avoid race conditions
        unsafe {
            std::env::set_var("CLAUDE_PROJECT_DIR", "");
        }

        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("test".to_string()),
                file_path: None,
            },
        };

        let ctx = Context::from_input(&input);

        // Should fallback to current_dir()
        let expected = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        assert_eq!(ctx.workspace_root, expected);

        // Restore the original value
        // SAFETY: This test is run with --test-threads=1 to avoid race conditions
        unsafe {
            match original {
                Some(val) => std::env::set_var("CLAUDE_PROJECT_DIR", val),
                None => std::env::remove_var("CLAUDE_PROJECT_DIR"),
            }
        }
    }
}
