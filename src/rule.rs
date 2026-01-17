use crate::config::{Config, RuleConfig};
use crate::context::Context;
use crate::error::{CchookedError, Result};
use regex_lite::Regex;

/// Hook event types that trigger rule evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    PreToolUse,
    PostToolUse,
}

impl EventType {
    /// Parses a string into an `EventType`.
    ///
    /// Returns an error if the string is not a valid event type.
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "PreToolUse" => Ok(EventType::PreToolUse),
            "PostToolUse" => Ok(EventType::PostToolUse),
            _ => Err(CchookedError::InvalidEventType {
                value: s.to_string(),
                valid: vec!["PreToolUse", "PostToolUse"],
            }),
        }
    }

    /// Returns the string representation of the event type.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::PreToolUse => "PreToolUse",
            EventType::PostToolUse => "PostToolUse",
        }
    }
}

/// Action types that can be performed when a rule matches.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    /// Blocks the tool execution with an optional message.
    Block,
    /// Transforms the tool input before execution.
    Transform,
    /// Runs an external command.
    Run,
    /// Logs the tool usage to a file.
    Log,
}

/// Log output format for the Log action.
#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    /// Plain text format.
    Text,
    /// JSON format.
    Json,
}

impl LogFormat {
    /// Parses a string into a `LogFormat`. Defaults to Text for unknown values.
    pub fn from_str(s: &str) -> Self {
        match s {
            "json" => LogFormat::Json,
            _ => LogFormat::Text,
        }
    }
}

/// Behavior when a Run action command fails.
#[derive(Debug, Clone, PartialEq)]
pub enum OnErrorBehavior {
    /// Ignore the error and continue.
    Ignore,
    /// Fail and block the tool execution.
    Fail,
}

impl OnErrorBehavior {
    /// Parses a string into `OnErrorBehavior`. Defaults to Ignore for unknown values.
    pub fn from_str(s: &str) -> Self {
        match s {
            "fail" => OnErrorBehavior::Fail,
            _ => OnErrorBehavior::Ignore,
        }
    }
}

impl ActionType {
    /// Parses a string into an `ActionType`.
    ///
    /// Returns an error if the string is not a valid action type.
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "block" => Ok(ActionType::Block),
            "transform" => Ok(ActionType::Transform),
            "run" => Ok(ActionType::Run),
            "log" => Ok(ActionType::Log),
            _ => Err(CchookedError::InvalidActionType {
                value: s.to_string(),
                valid: vec!["block", "transform", "run", "log"],
            }),
        }
    }
}

/// Conditional filters for rule matching.
#[derive(Debug, Default)]
pub struct WhenCondition {
    /// Regex patterns to match against the command.
    pub command_patterns: Vec<Regex>,
    /// Regex patterns to match against the file path.
    pub file_path_patterns: Vec<Regex>,
    /// Regex patterns to match against the current git branch.
    pub branch_patterns: Vec<Regex>,
}

/// Transform rule configuration for command replacement.
#[derive(Debug, Clone)]
pub struct TransformRule {
    /// Regex pattern to match in the command.
    pub command_pattern: Option<Regex>,
    /// Replacement string for the matched pattern.
    pub command_replacement: Option<String>,
}

/// A compiled rule ready for evaluation.
#[derive(Debug)]
pub struct Rule {
    /// Name identifier for the rule.
    pub name: String,
    /// Event type that triggers this rule.
    pub event: EventType,
    /// Regex pattern to match against tool names.
    pub matcher: Regex,
    /// Action to perform when the rule matches.
    pub action: ActionType,
    /// Priority for rule ordering (higher priority rules are evaluated first).
    pub priority: i32,
    /// Optional message for block actions.
    pub message: Option<String>,
    /// Additional conditions for matching.
    pub when: WhenCondition,
    /// Transform configuration for transform actions.
    pub transform: Option<TransformRule>,
    /// Command template for run actions.
    pub run_command: Option<String>,
    /// Behavior when run command fails.
    pub on_error: OnErrorBehavior,
    /// File path for log actions.
    pub log_file: Option<String>,
    /// Output format for log actions.
    pub log_format: LogFormat,
}

/// Rule evaluation result containing matched rule information.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MatchResult {
    /// Name of the matched rule.
    pub rule_name: String,
    /// Action to perform.
    pub action: ActionType,
    /// Optional message for block actions.
    pub message: Option<String>,
    /// Transform configuration if applicable.
    pub transform: Option<TransformRule>,
    /// Command to run if applicable.
    pub run_command: Option<String>,
    /// Behavior when command fails.
    pub on_error: OnErrorBehavior,
    /// Log file path if applicable.
    pub log_file: Option<String>,
    /// Log format if applicable.
    pub log_format: LogFormat,
}

fn compile_regex_with_context(pattern: &str, rule_name: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| CchookedError::RegexError {
        rule_name: rule_name.to_string(),
        pattern: pattern.to_string(),
        detail: e.to_string(),
    })
}

/// Compiles a single rule configuration into an executable Rule.
///
/// Validates and compiles all regex patterns in the rule configuration.
pub fn compile_rule(name: &str, config: &RuleConfig) -> Result<Rule> {
    let event = EventType::from_str(&config.event)?;
    let matcher = compile_regex_with_context(&config.matcher, name)?;
    let action = ActionType::from_str(&config.action)?;

    let mut when = WhenCondition::default();

    if let Some(when_config) = &config.when {
        if let Some(command) = &when_config.command {
            for pattern in command.to_vec() {
                when.command_patterns
                    .push(compile_regex_with_context(&pattern, name)?);
            }
        }
        if let Some(file_path) = &when_config.file_path {
            for pattern in file_path.to_vec() {
                when.file_path_patterns
                    .push(compile_regex_with_context(&pattern, name)?);
            }
        }
        if let Some(branch) = &when_config.branch {
            for pattern in branch.to_vec() {
                when.branch_patterns
                    .push(compile_regex_with_context(&pattern, name)?);
            }
        }
    }

    let transform = if let Some(transform_config) = &config.transform {
        if let Some(ref cmd) = transform_config.command {
            Some(TransformRule {
                command_pattern: Some(compile_regex_with_context(&cmd[0], name)?),
                command_replacement: Some(cmd[1].clone()),
            })
        } else {
            None
        }
    } else {
        None
    };

    if action == ActionType::Log && config.log_file.is_none() {
        return Err(CchookedError::LogFileMissing {
            rule_name: name.to_string(),
        });
    }

    Ok(Rule {
        name: name.to_string(),
        event,
        matcher,
        action,
        priority: config.priority,
        message: config.message.clone(),
        when,
        transform,
        run_command: config.command.clone(),
        on_error: OnErrorBehavior::from_str(&config.on_error),
        log_file: config.log_file.clone(),
        log_format: LogFormat::from_str(&config.log_format),
    })
}

/// Compiles all rules from a configuration.
///
/// Returns rules sorted by priority (highest first).
pub fn compile_rules(config: &Config) -> Result<Vec<Rule>> {
    let mut rules = Vec::new();

    for (name, rule_config) in &config.rules {
        rules.push(compile_rule(name, rule_config)?);
    }

    rules.sort_by(|a, b| b.priority.cmp(&a.priority));

    Ok(rules)
}

/// Input parameters for a tool invocation.
#[derive(Debug)]
pub struct ToolInput {
    /// Command string for Bash tool.
    pub command: Option<String>,
    /// File path for file-related tools.
    pub file_path: Option<String>,
}

/// Hook input containing tool name and parameters.
#[derive(Debug)]
pub struct HookInput {
    /// Name of the tool being invoked.
    pub tool_name: String,
    /// Input parameters for the tool.
    pub tool_input: ToolInput,
}

fn matches_command(patterns: &[Regex], command: &str) -> bool {
    if patterns.is_empty() {
        return true;
    }
    patterns.iter().any(|p| p.is_match(command))
}

fn matches_file_path(patterns: &[Regex], file_path: &str) -> bool {
    if patterns.is_empty() {
        return true;
    }
    patterns.iter().any(|p| p.is_match(file_path))
}

fn matches_branch(patterns: &[Regex], current_branch: &str) -> bool {
    if patterns.is_empty() {
        return true;
    }
    patterns.iter().any(|p| p.is_match(current_branch))
}

/// Evaluates rules against the given event and input.
///
/// Returns the first matching rule's result along with the context, or None if no rule matches.
pub fn evaluate_rules(
    rules: &[Rule],
    event: &EventType,
    input: &HookInput,
) -> Option<(MatchResult, Context)> {
    let mut context: Option<Context> = None;

    for rule in rules {
        if rule.event != *event {
            continue;
        }

        if !rule.matcher.is_match(&input.tool_name) {
            continue;
        }

        if !rule.when.command_patterns.is_empty() {
            let command = input.tool_input.command.as_deref().unwrap_or("");
            if !matches_command(&rule.when.command_patterns, command) {
                continue;
            }
        }

        if !rule.when.file_path_patterns.is_empty() {
            let file_path = input.tool_input.file_path.as_deref().unwrap_or("");
            if !matches_file_path(&rule.when.file_path_patterns, file_path) {
                continue;
            }
        }

        if !rule.when.branch_patterns.is_empty() {
            if context.is_none() {
                context = Some(Context::from_input(input));
            }
            if let Some(ref ctx) = context
                && !matches_branch(&rule.when.branch_patterns, &ctx.branch)
            {
                continue;
            }
        }

        // If context was not created during branch matching, create it now
        let ctx = context.unwrap_or_else(|| Context::from_input(input));

        return Some((
            MatchResult {
                rule_name: rule.name.clone(),
                action: rule.action.clone(),
                message: rule.message.clone(),
                transform: rule.transform.clone(),
                run_command: rule.run_command.clone(),
                on_error: rule.on_error.clone(),
                log_file: rule.log_file.clone(),
                log_format: rule.log_format.clone(),
            },
            ctx,
        ));
    }

    None
}
