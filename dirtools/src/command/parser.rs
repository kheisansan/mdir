// command/parser.rs - コマンド文字列パーサー
//
// `:` プロンプトに入力された文字列をトークン分割し、
// ビルトインコマンドまたはシェルコマンドとして構造化する。

/// パース済みビルトインコマンド
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// コマンド名 (小文字正規化済み)
    pub name: String,
    /// 引数リスト
    pub args: Vec<String>,
}

/// パース結果
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// ビルトインコマンド
    Builtin(ParsedCommand),
    /// シェルコマンド (:! プレフィックス)
    Shell(String),
    /// 空入力
    Empty,
}

/// コマンド文字列をパースする
///
/// # Examples
/// - `"help"` → `Builtin { name: "help", args: [] }`
/// - `"cd ~/projects"` → `Builtin { name: "cd", args: ["~/projects"] }`
/// - `"!ls -la"` → `Shell("ls -la")`
/// - `""` → `Empty`
pub fn parse(input: &str) -> ParseResult {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return ParseResult::Empty;
    }

    // シェルコマンド判定 (:! で始まる)
    if let Some(shell_cmd) = trimmed.strip_prefix('!') {
        return ParseResult::Shell(shell_cmd.trim().to_string());
    }

    // スペース区切りでトークン分割
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.is_empty() {
        return ParseResult::Empty;
    }

    ParseResult::Builtin(ParsedCommand {
        name: tokens[0].to_lowercase(),
        args: tokens[1..].iter().map(|s| s.to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert!(matches!(parse(""), ParseResult::Empty));
        assert!(matches!(parse("   "), ParseResult::Empty));
    }

    #[test]
    fn test_parse_help() {
        match parse("help") {
            ParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "help");
                assert!(cmd.args.is_empty());
            }
            _ => panic!("Expected Builtin"),
        }
    }

    #[test]
    fn test_parse_help_with_topic() {
        match parse("help keybindings") {
            ParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "help");
                assert_eq!(cmd.args, vec!["keybindings"]);
            }
            _ => panic!("Expected Builtin"),
        }
    }

    #[test]
    fn test_parse_cd() {
        match parse("cd ~/projects") {
            ParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "cd");
                assert_eq!(cmd.args, vec!["~/projects"]);
            }
            _ => panic!("Expected Builtin"),
        }
    }

    #[test]
    fn test_parse_shell_command() {
        match parse("!ls -la") {
            ParseResult::Shell(cmd) => assert_eq!(cmd, "ls -la"),
            _ => panic!("Expected Shell"),
        }
    }

    #[test]
    fn test_parse_case_insensitive() {
        match parse("HELP") {
            ParseResult::Builtin(cmd) => assert_eq!(cmd.name, "help"),
            _ => panic!("Expected Builtin"),
        }
    }
}
