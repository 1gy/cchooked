#![allow(clippy::unwrap_used)]

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn run_cchooked(event: &str, input: &str, config: &str) -> (i32, String, String) {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("hooks-rules.toml"), config).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_cchooked"))
        .arg(event)
        .current_dir(temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

#[test]
fn test_block_action() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "npm install express"}}"#;
    let config = r#"
[rules.no-npm]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "use bun"
when.command = "^npm\\s"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 2);
    assert!(stderr.contains("use bun"));
    assert!(stdout.is_empty());
}

#[test]
fn test_transform_action() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "npm install express"}}"#;
    let config = r#"
[rules.npm-to-bun]
event = "PreToolUse"
matcher = "Bash"
action = "transform"
when.command = "^npm\\s"
transform.command = ["^npm", "bun"]
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 0);
    assert!(stdout.contains("bun install express"));
    assert!(stdout.contains("hookSpecificOutput"));
    assert!(stderr.is_empty());
}

#[test]
fn test_no_match() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "bun install express"}}"#;
    let config = r#"
[rules.no-npm]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "use bun"
when.command = "^npm\\s"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_priority_order() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.low]
priority = 1
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "low"
when.command = ".*"

[rules.high]
priority = 10
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "high"
when.command = ".*"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 2);
    assert!(stderr.contains("high"));
    assert!(!stderr.contains("low"));
}

#[test]
fn test_file_path_condition() {
    let input = r#"{"tool_name": "Write", "tool_input": {"file_path": "/path/to/.env"}}"#;
    let config = r#"
[rules.protect-env]
event = "PreToolUse"
matcher = "Write"
action = "block"
message = "Cannot edit .env files"
when.file_path = ".*\\.env.*"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot edit .env files"));
}

#[test]
fn test_event_type_mismatch() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "npm install"}}"#;
    let config = r#"
[rules.post-only]
event = "PostToolUse"
matcher = "Bash"
action = "block"
message = "blocked"
when.command = "^npm\\s"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_matcher_regex() {
    let input = r#"{"tool_name": "Edit", "tool_input": {"file_path": "/test.txt"}}"#;
    let config = r#"
[rules.no-edit-write]
event = "PreToolUse"
matcher = "Edit|Write"
action = "block"
message = "editing disabled"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 2);
    assert!(stderr.contains("editing disabled"));
}

#[test]
fn test_invalid_json_input() {
    let input = "invalid json";
    let config = r#"
[rules.test]
event = "PreToolUse"
matcher = "Bash"
action = "block"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 1);
    assert!(stderr.contains("Error"));
}

#[test]
fn test_config_not_found() {
    let temp_dir = TempDir::new().unwrap();

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;

    let mut child = Command::new(env!("CARGO_BIN_EXE_cchooked"))
        .arg("PreToolUse")
        .current_dir(temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    assert_eq!(output.status.code().unwrap(), 0);
    assert!(String::from_utf8_lossy(&output.stderr).contains("Warning"));
}

#[test]
fn test_invalid_regex_pattern() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.invalid-regex]
event = "PreToolUse"
matcher = "[invalid(regex"
action = "block"
message = "should not reach"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 1);
    assert!(stderr.contains("Error") || stderr.contains("error") || stderr.contains("regex"));
}

#[test]
fn test_invalid_event_type() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.invalid-event]
event = "InvalidEvent"
matcher = "Bash"
action = "block"
message = "should not reach"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("Error")
            || stderr.contains("error")
            || stderr.contains("unknown")
            || stderr.contains("invalid")
    );
}

#[test]
fn test_invalid_action_type() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.invalid-action]
event = "PreToolUse"
matcher = "Bash"
action = "invalid_action"
message = "should not reach"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 1);
    assert!(
        stderr.contains("Error")
            || stderr.contains("error")
            || stderr.contains("unknown")
            || stderr.contains("invalid")
    );
}

fn run_cchooked_with_dir(
    event: &str,
    input: &str,
    config: &str,
    temp_dir: &TempDir,
) -> (i32, String, String) {
    let config_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("hooks-rules.toml"), config).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_cchooked"))
        .arg(event)
        .current_dir(temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

#[test]
fn test_log_action_requires_log_file() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo hello"}}"#;
    let config = r#"
[rules.log-bash]
event = "PreToolUse"
matcher = "Bash"
action = "log"
log_format = "text"
"#;

    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 1);
    assert!(stderr.contains("log_file") || stderr.contains("log action"));
}

#[test]
fn test_log_action_json_format_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("test-json.log");

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "ls -la"}}"#;
    let config = format!(
        r#"
[rules.log-json]
event = "PreToolUse"
matcher = "Bash"
action = "log"
log_format = "json"
log_file = "{}"
"#,
        log_file_path.to_str().unwrap()
    );

    let (exit_code, stdout, stderr) =
        run_cchooked_with_dir("PreToolUse", input, &config, &temp_dir);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty()); // ファイル出力時はstderrは空

    // ファイルの内容を確認
    let log_content = fs::read_to_string(&log_file_path).unwrap();
    assert!(log_content.contains(r#""event":"PreToolUse""#));
    assert!(log_content.contains(r#""tool":"Bash""#));
    assert!(log_content.contains(r#""command":"ls -la""#));
}

#[test]
fn test_log_action_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file_path = temp_dir.path().join("test.log");

    let input = r#"{"tool_name": "Write", "tool_input": {"file_path": "/tmp/test.txt"}}"#;
    let config = format!(
        r#"
[rules.log-to-file]
event = "PreToolUse"
matcher = "Write"
action = "log"
log_format = "text"
log_file = "{}"
"#,
        log_file_path.to_str().unwrap()
    );

    let (exit_code, stdout, stderr) =
        run_cchooked_with_dir("PreToolUse", input, &config, &temp_dir);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty()); // ファイル出力時はstderrは空

    // ファイルの内容を確認
    let log_content = fs::read_to_string(&log_file_path).unwrap();
    assert!(log_content.contains("PreToolUse"));
    assert!(log_content.contains("Write"));
    assert!(log_content.contains("/tmp/test.txt"));
}

#[test]
fn test_run_action_success() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.run-success]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "exit 0"
when.command = ".*"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_run_action_on_error_ignore() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.run-ignore]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "exit 1"
on_error = "ignore"
when.command = ".*"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_run_action_on_error_fail() {
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;
    let config = r#"
[rules.run-fail]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "echo 'error message' >&2 && exit 1"
on_error = "fail"
when.command = ".*"
"#;

    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input, config);

    assert_eq!(exit_code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Command failed"));
}

#[test]
fn test_when_command_array_or_logic() {
    let config = r#"
[rules.no-package-managers]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "use bun instead"
when.command = ["^npm\\s", "^yarn\\s", "^pnpm\\s"]
"#;

    // npm にマッチ
    let input_npm = r#"{"tool_name": "Bash", "tool_input": {"command": "npm install express"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_npm, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("use bun instead"));

    // yarn にマッチ
    let input_yarn = r#"{"tool_name": "Bash", "tool_input": {"command": "yarn add express"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_yarn, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("use bun instead"));

    // pnpm にマッチ
    let input_pnpm = r#"{"tool_name": "Bash", "tool_input": {"command": "pnpm install express"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_pnpm, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("use bun instead"));

    // bun はマッチしない
    let input_bun = r#"{"tool_name": "Bash", "tool_input": {"command": "bun install express"}}"#;
    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input_bun, config);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_when_file_path_array_or_logic() {
    let config = r#"
[rules.protect-sensitive-files]
event = "PreToolUse"
matcher = "Write"
action = "block"
message = "Cannot edit sensitive files"
when.file_path = [".*\\.env.*", ".*\\.secret.*", ".*/credentials\\.json$"]
"#;

    // .env にマッチ
    let input_env = r#"{"tool_name": "Write", "tool_input": {"file_path": "/path/to/.env"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_env, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot edit sensitive files"));

    // .env.local にマッチ
    let input_env_local =
        r#"{"tool_name": "Write", "tool_input": {"file_path": "/path/to/.env.local"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_env_local, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot edit sensitive files"));

    // .secret にマッチ
    let input_secret = r#"{"tool_name": "Write", "tool_input": {"file_path": "/config/.secret"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_secret, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot edit sensitive files"));

    // credentials.json にマッチ
    let input_creds =
        r#"{"tool_name": "Write", "tool_input": {"file_path": "/home/user/credentials.json"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_creds, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot edit sensitive files"));

    // 通常のファイルはマッチしない
    let input_normal =
        r#"{"tool_name": "Write", "tool_input": {"file_path": "/path/to/config.json"}}"#;
    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input_normal, config);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_when_command_and_file_path_and_logic() {
    let config = r#"
[rules.no-cat-env]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "Cannot cat .env files"
when.command = "^cat\\s"
when.file_path = ".*\\.env.*"
"#;

    let input_cat_env = r#"{"tool_name": "Bash", "tool_input": {"command": "cat .env", "file_path": "/path/to/.env"}}"#;
    let (exit_code, _, stderr) = run_cchooked("PreToolUse", input_cat_env, config);
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Cannot cat .env files"));

    let input_cat_other = r#"{"tool_name": "Bash", "tool_input": {"command": "cat config.json", "file_path": "/path/to/config.json"}}"#;
    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input_cat_other, config);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());

    let input_echo_env = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test", "file_path": "/path/to/.env"}}"#;
    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input_echo_env, config);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());

    let input_echo_other = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test", "file_path": "/path/to/config.json"}}"#;
    let (exit_code, stdout, stderr) = run_cchooked("PreToolUse", input_echo_other, config);
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

fn run_cchooked_with_branch(
    event: &str,
    input: &str,
    config: &str,
    branch: &str,
) -> (i32, String, String) {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("hooks-rules.toml"), config).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_cchooked"))
        .arg(event)
        .env("CCHOOKED_BRANCH", branch)
        .current_dir(temp_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (exit_code, stdout, stderr)
}

#[test]
fn test_when_branch_single_pattern() {
    let config = r#"
[rules.main-only]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "Blocked on main branch"
when.branch = "^main$"
"#;

    // main ブランチでマッチ
    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test"}}"#;
    let (exit_code, _, stderr) = run_cchooked_with_branch("PreToolUse", input, config, "main");
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Blocked on main branch"));

    // feature ブランチではマッチしない
    let (exit_code, stdout, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "feature/test");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_when_branch_array_or_logic() {
    let config = r#"
[rules.protected-branches]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "Blocked on protected branches"
when.branch = ["^main$", "^master$", "^release/.*"]
"#;

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test"}}"#;

    // main にマッチ
    let (exit_code, _, stderr) = run_cchooked_with_branch("PreToolUse", input, config, "main");
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Blocked on protected branches"));

    // master にマッチ
    let (exit_code, _, stderr) = run_cchooked_with_branch("PreToolUse", input, config, "master");
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Blocked on protected branches"));

    // release/v1.0 にマッチ
    let (exit_code, _, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "release/v1.0");
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Blocked on protected branches"));

    // feature ブランチはマッチしない
    let (exit_code, stdout, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "feature/new-feature");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_when_branch_no_match() {
    let config = r#"
[rules.feature-only]
event = "PreToolUse"
matcher = "Bash"
action = "block"
message = "Blocked on feature branches"
when.branch = "^feature/.*"
"#;

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test"}}"#;

    // main はマッチしない
    let (exit_code, stdout, stderr) = run_cchooked_with_branch("PreToolUse", input, config, "main");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());

    // develop はマッチしない
    let (exit_code, stdout, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "develop");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());

    // hotfix/xxx はマッチしない
    let (exit_code, stdout, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "hotfix/urgent-fix");
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());

    // feature/xxx はマッチする
    let (exit_code, _, stderr) =
        run_cchooked_with_branch("PreToolUse", input, config, "feature/new-feature");
    assert_eq!(exit_code, 2);
    assert!(stderr.contains("Blocked on feature branches"));
}

// =============================================================================
// working_dir テスト
// =============================================================================

#[test]
fn test_working_dir_default_file_dir() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("src").join("nested");
    fs::create_dir_all(&subdir).unwrap();

    let output_file = temp_dir.path().join("pwd_output.txt");
    let file_path = subdir.join("test.txt");

    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}", "file_path": "{}"}}}}"#,
        output_file.to_str().unwrap(),
        file_path.to_str().unwrap()
    );

    let config = r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
"#;

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, config, &temp_dir);

    assert_eq!(exit_code, 0);

    // pwd の出力が file_path の親ディレクトリと一致することを確認
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), subdir.to_str().unwrap());
}

#[test]
fn test_working_dir_explicit_workspace_root() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("src");
    fs::create_dir_all(&subdir).unwrap();

    let output_file = temp_dir.path().join("pwd_output.txt");
    let file_path = subdir.join("test.txt");

    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}", "file_path": "{}"}}}}"#,
        output_file.to_str().unwrap(),
        file_path.to_str().unwrap()
    );

    let config = r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
working_dir = "${workspace_root}"
"#;

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, config, &temp_dir);

    assert_eq!(exit_code, 0);

    // pwd の出力が workspace root (temp_dir) と一致することを確認
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), temp_dir.path().to_str().unwrap());
}

#[test]
fn test_working_dir_subdirectory() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("custom_dir");
    fs::create_dir_all(&subdir).unwrap();

    let output_file = temp_dir.path().join("pwd_output.txt");

    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}"}}}}"#,
        output_file.to_str().unwrap()
    );

    let config = format!(
        r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${{command}}"
working_dir = "${{workspace_root}}/custom_dir"
"#
    );

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, &config, &temp_dir);

    assert_eq!(exit_code, 0);

    // pwd の出力が指定したサブディレクトリと一致することを確認
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), subdir.to_str().unwrap());
}

#[test]
fn test_working_dir_relative_path() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("relative_subdir");
    fs::create_dir_all(&subdir).unwrap();

    let output_file = temp_dir.path().join("pwd_output.txt");

    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}"}}}}"#,
        output_file.to_str().unwrap()
    );

    // 相対パスを指定
    let config = r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
working_dir = "relative_subdir"
"#;

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, config, &temp_dir);

    assert_eq!(exit_code, 0);

    // 相対パスが workspace_root からの相対パスとして解決されることを確認
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), subdir.to_str().unwrap());
}

#[test]
fn test_file_dir_variable_in_command() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("src").join("lib");
    fs::create_dir_all(&subdir).unwrap();

    let output_file = temp_dir.path().join("file_dir_output.txt");
    let file_path = subdir.join("module.rs");

    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "test", "file_path": "{}"}}}}"#,
        file_path.to_str().unwrap()
    );

    let config = format!(
        r#"
[rules.echo-file-dir]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "echo -n ${{file_dir}} > {}"
working_dir = "${{workspace_root}}"
"#,
        output_file.to_str().unwrap()
    );

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, &config, &temp_dir);

    assert_eq!(exit_code, 0);

    // ${file_dir} が正しく展開されることを確認
    let file_dir_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(file_dir_output, subdir.to_str().unwrap());
}

#[test]
fn test_workspace_root_variable_in_command() {
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("workspace_root_output.txt");

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "test"}}"#;

    let config = format!(
        r#"
[rules.echo-workspace-root]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "echo -n ${{workspace_root}} > {}"
"#,
        output_file.to_str().unwrap()
    );

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", input, &config, &temp_dir);

    assert_eq!(exit_code, 0);

    // ${workspace_root} が正しく展開されることを確認
    let workspace_root_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(workspace_root_output, temp_dir.path().to_str().unwrap());
}

#[test]
fn test_working_dir_no_file_path() {
    // file_path が指定されていない場合（Bash ツールなど）
    // working_dir も未指定の場合、cchooked の CWD で実行されることを確認
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("pwd_output.txt");

    // file_path を指定しない入力
    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}"}}}}"#,
        output_file.to_str().unwrap()
    );

    // working_dir を指定しない設定
    let config = r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
"#;

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, config, &temp_dir);

    assert_eq!(exit_code, 0);

    // file_path がなく working_dir も未指定の場合、cchooked の CWD (temp_dir) で実行される
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), temp_dir.path().to_str().unwrap());
}

#[test]
fn test_working_dir_nonexistent_directory_ignore() {
    // 存在しないディレクトリを working_dir に指定
    // on_error = "ignore" の場合、エラーを無視して exit 0
    let temp_dir = TempDir::new().unwrap();

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test"}}"#;

    let config = r#"
[rules.run-in-nonexistent]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
working_dir = "/nonexistent/directory/that/does/not/exist"
on_error = "ignore"
"#;

    let (exit_code, stdout, stderr) = run_cchooked_with_dir("PreToolUse", input, config, &temp_dir);

    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
}

#[test]
fn test_working_dir_nonexistent_directory_fail() {
    // 存在しないディレクトリを working_dir に指定
    // on_error = "fail" の場合、exit 2 で "Working directory does not exist" エラー
    let temp_dir = TempDir::new().unwrap();

    let input = r#"{"tool_name": "Bash", "tool_input": {"command": "echo test"}}"#;

    let config = r#"
[rules.run-in-nonexistent]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
working_dir = "/nonexistent/directory/that/does/not/exist"
on_error = "fail"
"#;

    let (exit_code, stdout, stderr) = run_cchooked_with_dir("PreToolUse", input, config, &temp_dir);

    assert_eq!(exit_code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("Working directory does not exist"));
}

#[test]
fn test_working_dir_empty_expansion() {
    // working_dir = "${file_dir}" で file_path が空の場合
    // cchooked の CWD で実行されることを確認
    let temp_dir = TempDir::new().unwrap();
    let output_file = temp_dir.path().join("pwd_output.txt");

    // file_path を指定しない入力（file_dir は空になる）
    let input = format!(
        r#"{{"tool_name": "Bash", "tool_input": {{"command": "pwd > {}"}}}}"#,
        output_file.to_str().unwrap()
    );

    // working_dir = "${file_dir}" を指定（展開後は空文字列）
    let config = r#"
[rules.run-pwd]
event = "PreToolUse"
matcher = "Bash"
action = "run"
command = "${command}"
working_dir = "${file_dir}"
"#;

    let (exit_code, _, _) = run_cchooked_with_dir("PreToolUse", &input, config, &temp_dir);

    assert_eq!(exit_code, 0);

    // ${file_dir} が空で展開された場合、cchooked の CWD (temp_dir) で実行される
    let pwd_output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(pwd_output.trim(), temp_dir.path().to_str().unwrap());
}
