use shellish_parse::multiparse;

/// 複合コマンドを分割して個々のコマンドを返す
///
/// セパレータ: &&, ||, ;, |
/// サブシェル `()` は非対応
pub fn split_compound_command(command: &str) -> Vec<Vec<String>> {
    if command.trim().is_empty() {
        return Vec::new();
    }

    let separators = ["&&", "||", ";", "|"];

    match multiparse(command, false, &separators) {
        Ok(commands) => commands
            .into_iter()
            .map(|(args, _sep)| args)
            .filter(|args| !args.is_empty())
            .collect(),
        Err(_) => {
            vec![vec![command.to_string()]]
        }
    }
}

/// コマンドリストを文字列として再構築（マッチング用）
pub fn commands_to_strings(commands: &[Vec<String>]) -> Vec<String> {
    commands.iter().map(|args| args.join(" ")).collect()
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_command() {
        assert!(split_compound_command("").is_empty());
        assert!(split_compound_command("   ").is_empty());
    }

    #[test]
    fn test_simple_command() {
        let result = split_compound_command("echo hello");
        assert_eq!(result, vec![vec!["echo", "hello"]]);
    }

    #[test]
    fn test_and_operator() {
        let result = split_compound_command("git status && git push --force");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["git", "status"]);
        assert_eq!(result[1], vec!["git", "push", "--force"]);
    }

    #[test]
    fn test_or_operator() {
        let result = split_compound_command("test -f file || echo 'not found'");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["test", "-f", "file"]);
        assert_eq!(result[1], vec!["echo", "not found"]);
    }

    #[test]
    fn test_semicolon_separator() {
        let result = split_compound_command("echo a; echo b; echo c");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], vec!["echo", "a"]);
        assert_eq!(result[1], vec!["echo", "b"]);
        assert_eq!(result[2], vec!["echo", "c"]);
    }

    #[test]
    fn test_pipe_operator() {
        let result = split_compound_command("cat file.txt | grep pattern");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["cat", "file.txt"]);
        assert_eq!(result[1], vec!["grep", "pattern"]);
    }

    #[test]
    fn test_quoted_string_preserved() {
        // クォート内のセパレータは分割されない
        let result = split_compound_command("echo 'a && b'");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec!["echo", "a && b"]);
    }

    #[test]
    fn test_double_quoted_string_preserved() {
        let result = split_compound_command(r#"echo "git push --force""#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec!["echo", "git push --force"]);
    }

    #[test]
    fn test_comment_ignored() {
        // shellish_parse はコメントを処理する
        let result = split_compound_command("echo hello # this is a comment");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec!["echo", "hello"]);
    }

    #[test]
    fn test_commands_to_strings() {
        let commands = vec![
            vec!["git".to_string(), "status".to_string()],
            vec!["git".to_string(), "push".to_string(), "--force".to_string()],
        ];
        let strings = commands_to_strings(&commands);
        assert_eq!(strings, vec!["git status", "git push --force"]);
    }

    #[test]
    fn test_mixed_operators() {
        let result = split_compound_command("cmd1 && cmd2 || cmd3; cmd4 | cmd5");
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], vec!["cmd1"]);
        assert_eq!(result[1], vec!["cmd2"]);
        assert_eq!(result[2], vec!["cmd3"]);
        assert_eq!(result[3], vec!["cmd4"]);
        assert_eq!(result[4], vec!["cmd5"]);
    }

    #[test]
    fn test_complex_command_with_args() {
        let result = split_compound_command("npm install -g typescript && npm run build --prod");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec!["npm", "install", "-g", "typescript"]);
        assert_eq!(result[1], vec!["npm", "run", "build", "--prod"]);
    }
}
