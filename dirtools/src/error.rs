// error.rs - アプリケーション全体のエラー型定義
//
// 全モジュール共通のエラー型 `AppError` を提供する。
// thiserror による derive マクロで Display/Error トレイトを自動実装し、
// ユーザー向けメッセージ生成もここに集約する。

use std::path::PathBuf;

/// アプリケーション全体のエラー型
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum AppError {
    // --- I/O エラー ---
    #[error("I/O エラー: {0}")]
    Io(#[from] std::io::Error),

    // --- ファイル操作エラー ---
    #[error("既に存在します: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("パスが見つかりません: {0}")]
    PathNotFound(PathBuf),

    #[error("ディレクトリではありません: {0}")]
    NotADirectory(PathBuf),

    #[error("無効なパスです")]
    InvalidPath,

    #[error("権限がありません: {0}")]
    PermissionDenied(PathBuf),

    #[error("ゴミ箱操作に失敗しました: {0}")]
    TrashError(String),

    // --- コマンドエラー ---
    #[error("不明なコマンド: '{0}'。:help で利用可能なコマンドを確認してください。")]
    UnknownCommand(String),

    #[error("'{command}' の引数が不正です: {expected} 個必要、{got} 個指定")]
    InvalidArguments {
        command: String,
        expected: String,
        got: usize,
    },

    #[error("無効なソート基準: '{0}'。name, size, date, ext を使用してください。")]
    InvalidSortCriteria(String),

    #[error("不明なオプション: '{0}'")]
    UnknownOption(String),

    // --- 環境エラー ---
    #[error("ホームディレクトリが見つかりません")]
    HomeDirNotFound,

    #[error("ターミナルが小さすぎます: 最小 {min_w}x{min_h}、現在 {actual_w}x{actual_h}")]
    TerminalTooSmall {
        min_w: u16,
        min_h: u16,
        actual_w: u16,
        actual_h: u16,
    },

    // --- 設定エラー ---
    #[error("設定ファイルのパースエラー: {0}")]
    ConfigError(String),
}

impl AppError {
    /// ステータスバーやコマンドラインに表示するユーザー向け短縮メッセージ
    pub fn user_message(&self) -> String {
        match self {
            Self::UnknownCommand(cmd) => {
                format!("エラー: 不明なコマンド '{}'。:help で確認してください。", cmd)
            }
            Self::PathNotFound(p) => format!("エラー: パスが見つかりません: {}", p.display()),
            Self::PermissionDenied(p) => format!("エラー: 権限がありません: {}", p.display()),
            Self::FileAlreadyExists(p) => format!("エラー: 既に存在します: {}", p.display()),
            Self::InvalidArguments { command, expected, got } => {
                format!("エラー: '{}' は引数 {} 個必要（{} 個指定）", command, expected, got)
            }
            Self::InvalidSortCriteria(s) => {
                format!("エラー: 無効なソート '{}' → name, size, date, ext", s)
            }
            _ => format!("エラー: {}", self),
        }
    }
}
