mod action;
mod config;
mod context;
mod error;
mod output;
mod parser;
mod rule;

use error::CchookedError;
use rule::{EventType, HookInput, ToolInput};
use serde::Deserialize;
use std::env;
use std::io::{self, Read};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Args {
    event: Option<String>,
    config_path: Option<String>,
    show_help: bool,
    show_version: bool,
}

#[derive(Debug, Deserialize)]
struct RawHookInput {
    tool_name: String,
    tool_input: RawToolInput,
}

#[derive(Debug, Deserialize)]
struct RawToolInput {
    command: Option<String>,
    file_path: Option<String>,
    #[serde(flatten)]
    _extra: serde_json::Value, // Ignore other fields
}

impl From<RawHookInput> for HookInput {
    fn from(raw: RawHookInput) -> Self {
        HookInput {
            tool_name: raw.tool_name,
            tool_input: ToolInput {
                command: raw.tool_input.command,
                file_path: raw.tool_input.file_path,
            },
        }
    }
}

fn print_help() {
    eprintln!(
        r#"cchooked - Claude Code Hooks Engine

USAGE:
    cchooked <EVENT> [OPTIONS]

ARGUMENTS:
    <EVENT>    Event type: PreToolUse or PostToolUse

OPTIONS:
    --config <PATH>    Path to config file (default: .claude/hooks-rules.toml)
    --help, -h         Show this help message
    --version, -v      Show version

EXAMPLES:
    echo '{{"tool_name":"Bash","tool_input":{{"command":"npm install"}}}}' | cchooked PreToolUse
    cchooked PreToolUse --config /path/to/hooks-rules.toml < input.json"#
    );
}

fn print_version() {
    eprintln!("cchooked {VERSION}");
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut result = Args {
        event: None,
        config_path: None,
        show_help: false,
        show_version: false,
    };

    let mut i = 0;
    while i < args.len() {
        if let Some(arg) = args.get(i) {
            match arg.as_str() {
                "--help" | "-h" => result.show_help = true,
                "--version" | "-v" => result.show_version = true,
                "--config" => {
                    i += 1;
                    if let Some(config_arg) = args.get(i) {
                        result.config_path = Some(config_arg.clone());
                    } else {
                        eprintln!("Error: --config option requires a value");
                        std::process::exit(1);
                    }
                }
                a if !a.starts_with('-') && result.event.is_none() => {
                    result.event = Some(a.to_string());
                }
                a if a.starts_with('-') => {
                    eprintln!("Warning: unknown argument '{a}'");
                }
                _ => {}
            }
        }
        i += 1;
    }

    result
}

fn read_input() -> error::Result<HookInput> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let raw: RawHookInput = serde_json::from_str(&buffer)?;
    Ok(raw.into())
}

fn run() -> error::Result<output::Output> {
    let args = parse_args();

    if args.show_help {
        print_help();
        return Ok(output::no_match_output());
    }

    if args.show_version {
        print_version();
        return Ok(output::no_match_output());
    }

    let event_str = args.event.ok_or_else(|| {
        CchookedError::InputParseError(
            "Missing event argument. Usage: cchooked <EVENT>".to_string(),
        )
    })?;

    let event = EventType::from_str(&event_str)?;
    let input = read_input()?;
    let config = config::load_config(args.config_path.as_deref())?;
    let rules = rule::compile_rules(&config)?;

    match rule::evaluate_rules(&rules, &event, &input) {
        Some((match_result, context)) => {
            Ok(action::execute_action(&match_result, &context, &event))
        }
        None => Ok(output::no_match_output()),
    }
}

fn main() {
    let result = run();

    match result {
        Ok(out) => {
            output::emit(&out);
            std::process::exit(out.exit_code);
        }
        Err(e) => {
            if let CchookedError::ConfigNotFound(ref _path) = e {
                eprintln!("Warning: {e}");
                let out = output::no_match_output();
                output::emit(&out);
                std::process::exit(0);
            } else {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    }
}
