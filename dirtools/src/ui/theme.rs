// ui/theme.rs - カラーテーマ定義
//
// Tokyo Night 風のデフォルトテーマ。
// 各 UI 要素のスタイルを一箇所で管理し、全ウィジェットから共通参照する。

use ratatui::style::{Color, Modifier, Style};

// ---------------------------------------------------------------------------
// ベースカラー
// ---------------------------------------------------------------------------
pub const BG: Color = Color::Rgb(26, 27, 38);
pub const FG: Color = Color::Rgb(192, 202, 245);
pub const FG_DIM: Color = Color::Rgb(86, 95, 137);
pub const BG_SELECTED: Color = Color::Rgb(40, 52, 87);
pub const BG_STATUS: Color = Color::Rgb(31, 35, 53);

// ---------------------------------------------------------------------------
// セマンティックカラー
// ---------------------------------------------------------------------------
pub const BLUE: Color = Color::Rgb(122, 162, 247);
pub const GREEN: Color = Color::Rgb(158, 206, 106);
pub const CYAN: Color = Color::Rgb(125, 207, 255);
pub const YELLOW: Color = Color::Rgb(224, 175, 104);
pub const RED: Color = Color::Rgb(247, 118, 142);
pub const ORANGE: Color = Color::Rgb(255, 158, 100);
#[allow(dead_code)]
pub const TEAL: Color = Color::Rgb(115, 218, 202);

// ---------------------------------------------------------------------------
// スタイルファクトリ（共通部品）
// ---------------------------------------------------------------------------

/// ファイル種別に応じたスタイル
pub fn entry_style(
    is_dir: bool,
    is_exec: bool,
    is_symlink: bool,
    is_hidden: bool,
) -> Style {
    if is_hidden {
        Style::default().fg(FG_DIM)
    } else if is_dir {
        Style::default().fg(BLUE).add_modifier(Modifier::BOLD)
    } else if is_symlink {
        Style::default().fg(CYAN)
    } else if is_exec {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    }
}

/// マーク済みエントリのスタイル
pub fn marked_style() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

/// カーソル行のスタイル（アクティブペイン）
pub fn cursor_style() -> Style {
    Style::default().bg(BG_SELECTED).add_modifier(Modifier::BOLD)
}

/// カーソル行のスタイル（非アクティブペイン）
pub fn cursor_inactive_style() -> Style {
    Style::default().bg(Color::Rgb(51, 51, 51))
}

/// ヘッダーバーのスタイル
pub fn header_style() -> Style {
    Style::default().fg(FG).bg(BG_STATUS)
}

/// ステータスバーのスタイル
pub fn status_style() -> Style {
    Style::default().fg(FG).bg(BG_STATUS)
}

/// ファンクションバーのスタイル
pub fn function_bar_style() -> Style {
    Style::default().fg(FG_DIM).bg(BG)
}

/// ファンクションキーラベルのスタイル
pub fn function_key_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(FG_DIM)
        .add_modifier(Modifier::BOLD)
}

/// コマンド行プロンプトのスタイル
pub fn command_prompt_style() -> Style {
    Style::default().fg(FG).add_modifier(Modifier::BOLD)
}

/// エラーメッセージのスタイル
pub fn error_style() -> Style {
    Style::default().fg(RED).add_modifier(Modifier::BOLD)
}

/// 成功メッセージのスタイル
pub fn success_style() -> Style {
    Style::default().fg(GREEN)
}

/// 情報メッセージのスタイル
pub fn info_style() -> Style {
    Style::default().fg(CYAN)
}

/// パスバーのスタイル
pub fn path_style(is_active: bool) -> Style {
    if is_active {
        Style::default().fg(FG).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(FG_DIM)
    }
}

/// ペイン枠線のスタイル
pub fn border_style(is_active: bool) -> Style {
    if is_active {
        Style::default().fg(FG)
    } else {
        Style::default().fg(FG_DIM)
    }
}

/// ダイアログタイトルのスタイル
pub fn dialog_title_style() -> Style {
    Style::default()
        .fg(FG)
        .add_modifier(Modifier::BOLD)
}
