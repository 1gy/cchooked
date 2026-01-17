use crate::rule::HookInput;
use std::process::Command;

/// Execution context containing extracted input values and environment information.
#[derive(Debug)]
pub struct Context {
    /// Command string from tool input.
    pub command: String,
    /// File path from tool input.
    pub file_path: String,
    /// Name of the tool being invoked.
    pub tool_name: String,
    /// Current git branch name.
    pub branch: String,
}

impl Context {
    /// Creates a new context from hook input.
    ///
    /// Extracts command, file path, tool name, and detects the current git branch.
    pub fn from_input(input: &HookInput) -> Self {
        Self {
            command: input.tool_input.command.clone().unwrap_or_default(),
            file_path: input.tool_input.file_path.clone().unwrap_or_default(),
            tool_name: input.tool_name.clone(),
            branch: get_current_branch().unwrap_or_default(),
        }
    }

    /// Expands template variables in a string.
    ///
    /// Replaces `${command}`, `${file_path}`, `${tool_name}`, and `${branch}` with their values.
    pub fn expand(&self, template: &str) -> String {
        template
            .replace("${command}", &self.command)
            .replace("${file_path}", &self.file_path)
            .replace("${tool_name}", &self.tool_name)
            .replace("${branch}", &self.branch)
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
    }

    #[test]
    fn test_expand_variables() {
        let ctx = Context {
            command: "npm test".to_string(),
            file_path: "/src/main.rs".to_string(),
            tool_name: "Bash".to_string(),
            branch: "main".to_string(),
        };

        let result = ctx.expand("Running ${command} on ${branch}");
        assert_eq!(result, "Running npm test on main");

        let result = ctx.expand("File: ${file_path}, Tool: ${tool_name}");
        assert_eq!(result, "File: /src/main.rs, Tool: Bash");
    }

    #[test]
    fn test_expand_no_variables() {
        let ctx = Context {
            command: "test".to_string(),
            file_path: "".to_string(),
            tool_name: "Bash".to_string(),
            branch: "main".to_string(),
        };

        let result = ctx.expand("No variables here");
        assert_eq!(result, "No variables here");
    }
}
