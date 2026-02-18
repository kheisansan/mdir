// command/executor.rs - コマンド実行エンジン
//
// パース済みコマンドを受け取り、バリデーション → Action 生成を行う。
// ビルトインコマンドの定義もここに集約する。

use super::parser::{ParseResult, ParsedCommand};
use crate::error::AppError;
use crate::fs::entry::SortCriteria;
use std::path::PathBuf;

/// コマンド実行結果として返すアクション
#[derive(Debug)]
pub enum CommandAction {
    /// ヘルプ画面を表示
    ShowHelp(Option<String>),
    /// アプリ終了
    Quit,
    /// ディレクトリ移動
    ChangeDir(PathBuf),
    /// ソート変更
    SetSort(SortCriteria),
    /// 隠しファイル表示切替
    ToggleHidden,
    /// シェルコマンド実行
    Shell(String),
    /// ディレクトリ作成
    CreateDir(String),
    /// ファイル作成
    CreateFile(String),
    /// ブックマーク追加（将来）
    Bookmark(#[allow(dead_code)] Option<String>),
    /// ファイル名検索
    Find(String),
}

/// パース結果からコマンドを実行し、アクションを返す
pub fn execute(parse_result: &ParseResult, current_dir: &std::path::Path) -> Result<CommandAction, AppError> {
    match parse_result {
        ParseResult::Empty => Err(AppError::UnknownCommand(String::new())),
        ParseResult::Shell(cmd) => Ok(CommandAction::Shell(cmd.clone())),
        ParseResult::Builtin(cmd) => execute_builtin(cmd, current_dir),
    }
}

/// ビルトインコマンドの実行
fn execute_builtin(cmd: &ParsedCommand, current_dir: &std::path::Path) -> Result<CommandAction, AppError> {
    // エイリアス解決
    let name = resolve_alias(&cmd.name);

    match name.as_str() {
        "help" => {
            let topic = cmd.args.first().cloned();
            Ok(CommandAction::ShowHelp(topic))
        }
        "quit" => Ok(CommandAction::Quit),
        "cd" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "cd".to_string(),
                    expected: "1".to_string(),
                    got: 0,
                });
            }
            let path = resolve_path(&cmd.args[0], current_dir)?;
            Ok(CommandAction::ChangeDir(path))
        }
        "sort" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "sort".to_string(),
                    expected: "1".to_string(),
                    got: 0,
                });
            }
            let criteria = parse_sort_criteria(&cmd.args[0])?;
            Ok(CommandAction::SetSort(criteria))
        }
        "set" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "set".to_string(),
                    expected: "1-2".to_string(),
                    got: 0,
                });
            }
            match cmd.args[0].as_str() {
                "hidden" => Ok(CommandAction::ToggleHidden),
                other => Err(AppError::UnknownOption(other.to_string())),
            }
        }
        "mkdir" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "mkdir".to_string(),
                    expected: "1".to_string(),
                    got: 0,
                });
            }
            Ok(CommandAction::CreateDir(cmd.args[0].clone()))
        }
        "touch" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "touch".to_string(),
                    expected: "1".to_string(),
                    got: 0,
                });
            }
            Ok(CommandAction::CreateFile(cmd.args[0].clone()))
        }
        "bookmark" => {
            let name = cmd.args.first().cloned();
            Ok(CommandAction::Bookmark(name))
        }
        "find" => {
            if cmd.args.is_empty() {
                return Err(AppError::InvalidArguments {
                    command: "find".to_string(),
                    expected: "1".to_string(),
                    got: 0,
                });
            }
            // スペースを含む検索文字列に対応するため全引数を結合
            let query = cmd.args.join(" ");
            Ok(CommandAction::Find(query))
        }
        _ => Err(AppError::UnknownCommand(cmd.name.clone())),
    }
}

/// エイリアスを正式名に解決
fn resolve_alias(name: &str) -> String {
    match name {
        "q" => "quit".to_string(),
        "bm" => "bookmark".to_string(),
        other => other.to_string(),
    }
}

/// パス文字列を絶対パスに解決（~ 展開・相対パス解決）
fn resolve_path(path_str: &str, current_dir: &std::path::Path) -> Result<PathBuf, AppError> {
    let expanded = if let Some(rest) = path_str.strip_prefix('~') {
        let home = dirs::home_dir().ok_or(AppError::HomeDirNotFound)?;
        home.join(rest.trim_start_matches('/'))
    } else if path_str.starts_with('/') {
        PathBuf::from(path_str)
    } else {
        current_dir.join(path_str)
    };

    let canonical = expanded
        .canonicalize()
        .map_err(|_| AppError::PathNotFound(expanded.clone()))?;

    if !canonical.is_dir() {
        return Err(AppError::NotADirectory(canonical));
    }

    Ok(canonical)
}

/// ソート基準文字列をパース
fn parse_sort_criteria(s: &str) -> Result<SortCriteria, AppError> {
    match s.to_lowercase().as_str() {
        "name" | "n" => Ok(SortCriteria::Name),
        "size" | "s" => Ok(SortCriteria::Size),
        "date" | "d" | "time" | "modified" => Ok(SortCriteria::Date),
        "ext" | "e" | "extension" => Ok(SortCriteria::Extension),
        _ => Err(AppError::InvalidSortCriteria(s.to_string())),
    }
}
