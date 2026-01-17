/// Tool input parameters (temporary stub for context.rs dependency).
#[derive(Debug)]
#[allow(dead_code)]
pub struct ToolInput {
    /// Command string for Bash tool.
    pub command: Option<String>,
    /// File path for file-related tools.
    pub file_path: Option<String>,
}

/// Hook input containing tool name and parameters (temporary stub for context.rs dependency).
#[derive(Debug)]
#[allow(dead_code)]
pub struct HookInput {
    /// Name of the tool being invoked.
    pub tool_name: String,
    /// Input parameters for the tool.
    pub tool_input: ToolInput,
}
