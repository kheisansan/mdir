// ui/mod.rs - UI 描画モジュール
//
// メイン描画関数 render() がモードに応じた描画を統合する。
// 各サブモジュールは独立したウィジェットとして描画を担当する。

mod command_line;
mod dialog;
mod function_bar;
mod header;
mod help;
pub mod layout;
mod pane;
mod statusbar;
pub mod theme;
mod welcome;

use crate::app::App;
use crate::mode::AppMode;
use layout::AppLayout;
use ratatui::Frame;

/// メイン描画関数 - モードに応じて描画内容を切り替える
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // ターミナルサイズを保持（マウスヒット判定用）
    app.terminal_size = (area.width, area.height);

    // レイアウト計算
    let layout = AppLayout::calculate(area, app.pane_ratio);
    let visible_rows = layout.pane_visible_rows();

    // スクロール位置を調整
    app.left_pane.adjust_scroll(visible_rows);
    app.right_pane.adjust_scroll(visible_rows);

    // 1. ヘッダーバー
    header::render(frame, layout.header);

    // 2. パスバー（左右）
    let is_left_active = app.active_pane == crate::app::PaneSide::Left;
    pane::render_path_bar(frame, layout.left_path, &app.left_pane, is_left_active);
    pane::render_path_bar(frame, layout.right_path, &app.right_pane, !is_left_active);

    // 3. ファイルペイン（左右）
    let date_fmt = app.config.ui.date_format.clone();
    let time_fmt = app.config.ui.time_format.clone();
    pane::render_file_list(
        frame,
        layout.left_pane,
        &app.left_pane,
        is_left_active,
        &date_fmt,
        &time_fmt,
    );
    pane::render_file_list(
        frame,
        layout.right_pane,
        &app.right_pane,
        !is_left_active,
        &date_fmt,
        &time_fmt,
    );

    // 4. ステータスバー
    statusbar::render(frame, layout.statusbar, app);

    // 5. 最下行（モードによって切り替え）
    match app.mode {
        AppMode::Normal | AppMode::Dialog => {
            let has_git = app.active_pane_ref().git_branch.is_some();
            function_bar::render(frame, layout.bottom_bar, app.find_state.is_some(), has_git);
        }
        AppMode::Command => {
            command_line::render(frame, layout.bottom_bar, &app.command_state);
        }
        AppMode::Help => {} // ヘルプは全画面オーバーレイなので最下行は不要
    }

    // 6. 全画面オーバーレイ（ヘルプ画面）
    if app.mode == AppMode::Help {
        help::render(frame, area, &app.help_state);
    }

    // 7. ダイアログ（最前面オーバーレイ）
    if let Some(ref dialog) = app.dialog {
        dialog::render(frame, area, dialog);
    }

    // 8. ウェルカムメッセージ（初回起動時）
    if app.show_welcome {
        welcome::render(frame, area);
    }
}
