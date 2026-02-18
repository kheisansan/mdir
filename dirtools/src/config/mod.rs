// config/mod.rs - 設定管理モジュール
//
// ~/.config/mdir/config.toml からユーザー設定を読み込む。
// ファイルが存在しない場合はデフォルト値で動作する。

use crate::error::AppError;
use serde::Deserialize;
use std::path::PathBuf;

/// アプリケーション設定ルート
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub ui: UiConfig,
}

/// 一般設定
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// 隠しファイルの初期表示
    pub show_hidden: bool,
    /// デフォルトソート (name/size/date/ext)
    pub default_sort: String,
    /// ソート昇順
    pub sort_ascending: bool,
    /// ディレクトリ優先
    pub dirs_first: bool,
    /// 削除確認ダイアログ表示
    pub confirm_delete: bool,
    /// ゴミ箱を使用
    pub use_trash: bool,
}

/// UI 設定
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// ペイン幅比率 (0.2 - 0.8)
    pub pane_ratio: f32,
    /// スクロール余白行数
    pub scroll_margin: usize,
    /// 日付フォーマット
    pub date_format: String,
    /// 時刻フォーマット
    pub time_format: String,
    /// ウェルカムメッセージ表示
    pub show_welcome: bool,
    /// マウス有効
    pub mouse_enabled: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            default_sort: "name".to_string(),
            sort_ascending: true,
            dirs_first: true,
            confirm_delete: true,
            use_trash: true,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            pane_ratio: 0.5,
            scroll_margin: 3,
            date_format: "%m/%d".to_string(),
            time_format: "%H:%M".to_string(),
            show_welcome: true,
            mouse_enabled: true,
        }
    }
}

impl Config {
    /// 設定ファイルの標準パスを取得
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mdir")
    }

    /// 設定ファイルを読み込む。存在しなければデフォルトを返す。
    pub fn load(config_path: Option<&PathBuf>) -> Result<Self, AppError> {
        let path = config_path
            .cloned()
            .unwrap_or_else(|| Self::config_dir().join("config.toml"));

        if path.exists() {
            let content =
                std::fs::read_to_string(&path).map_err(|e| AppError::ConfigError(e.to_string()))?;
            let config: Config =
                toml::from_str(&content).map_err(|e| AppError::ConfigError(e.to_string()))?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// 設定ディレクトリが初回（存在しない）かどうか
    pub fn is_first_launch() -> bool {
        !Self::config_dir().exists()
    }
}
