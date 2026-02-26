// ui/dialog.rs - ダイアログ描画
//
// 確認ダイアログ・入力ダイアログを画面中央にオーバーレイ描画する。
// 全ダイアログに共通のフレーム描画ヘルパーを提供する。
//
// 入力ダイアログは unicode_width を使った表示幅計算と、
// テキストがダイアログ幅を超える場合のスクロール表示に対応する。

use super::theme;
use crate::app::DialogState;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;

/// ダイアログを描画
pub fn render(frame: &mut Frame, area: Rect, dialog: &DialogState) {
    match dialog {
        DialogState::Confirm {
            title,
            message,
            focus_yes,
            ..
        } => render_confirm(frame, area, title, message, *focus_yes),
        DialogState::Input {
            title,
            value,
            cursor_pos,
            ..
        } => render_input(frame, area, title, value, *cursor_pos),
        DialogState::Message { title, content } => render_message(frame, area, title, content),
        DialogState::BranchList {
            branches,
            filter_string,
            search_input,
            cursor,
            scroll_offset,
        } => render_branch_list(frame, area, branches, filter_string, search_input, *cursor, *scroll_offset),
    }
}

/// 確認ダイアログ描画
fn render_confirm(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    focus_yes: bool,
) {
    let dialog_width = 44u16.min(area.width.saturating_sub(4));
    let dialog_height = 7u16;
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // 背景クリア
    frame.render_widget(Clear, dialog_area);

    // 枠
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .title_style(theme::dialog_title_style().fg(theme::RED))
        .border_style(theme::border_style(true));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // メッセージ
    let msg = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme::FG));
    let msg_area = Rect::new(inner.x, inner.y + 1, inner.width, 2);
    frame.render_widget(msg, msg_area);

    // ボタン
    let yes_style = if focus_yes {
        Style::default().fg(theme::RED).add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(theme::FG_DIM)
    };
    let no_style = if !focus_yes {
        Style::default().fg(theme::GREEN).add_modifier(Modifier::REVERSED)
    } else {
        Style::default().fg(theme::FG_DIM)
    };

    let buttons = Line::from(vec![
        Span::raw("    "),
        Span::styled(" はい (y) ", yes_style),
        Span::raw("   "),
        Span::styled(" いいえ (n) ", no_style),
        Span::raw("    "),
    ]);
    let btn_area = Rect::new(inner.x, inner.y + 3, inner.width, 1);
    frame.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        btn_area,
    );
}

/// バイトオフセット → 表示カラム幅に変換
///
/// 文字列の先頭から指定バイト位置までの表示幅（カラム数）を計算する。
/// CJK 文字は2カラム、ASCII は1カラムとして正しく計算される。
fn byte_offset_to_display_width(s: &str, byte_offset: usize) -> usize {
    let safe_end = byte_offset.min(s.len());
    s[..safe_end]
        .chars()
        .map(|c| c.width().unwrap_or(0))
        .sum()
}

/// 入力ダイアログ描画
///
/// - unicode_width でカーソル位置を正しいカラムに配置
/// - テキストがダイアログ幅を超える場合、カーソル周辺をスクロール表示
fn render_input(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: &str,
    cursor_pos: usize,
) {
    let dialog_width = 44u16.min(area.width.saturating_sub(4));
    let dialog_height = 7u16;
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .title_style(theme::dialog_title_style())
        .border_style(theme::border_style(true));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    // 入力フィールドの表示可能幅（左余白1 + カーソル1 を除く）
    let field_width = inner.width.saturating_sub(2) as usize;

    // カーソル位置の表示幅を計算
    let cursor_display_col = byte_offset_to_display_width(value, cursor_pos);
    let total_display_width = byte_offset_to_display_width(value, value.len());

    // スクロールオフセット（表示幅ベース）: カーソルが常に見えるように調整
    let scroll_offset = if cursor_display_col <= field_width {
        0
    } else {
        cursor_display_col.saturating_sub(field_width)
    };

    // スクロールオフセットから表示する部分文字列を構築
    let mut display_text = String::new();
    let mut current_width = 0;
    let mut skipped_width = 0;

    for c in value.chars() {
        let char_width = c.width().unwrap_or(0);

        // スクロール分をスキップ
        if skipped_width < scroll_offset {
            skipped_width += char_width;
            continue;
        }

        // 表示幅を超えたら終了
        if current_width + char_width > field_width {
            break;
        }

        display_text.push(c);
        current_width += char_width;
    }

    // カーソルの表示上のカラム位置（スクロール分を引く）
    let cursor_col_on_screen = cursor_display_col.saturating_sub(scroll_offset);

    // 左端にスクロールインジケータ
    let prefix = if scroll_offset > 0 { "‹" } else { " " };
    // 右端にスクロールインジケータ
    let suffix = if total_display_width > scroll_offset + field_width { "›" } else { "" };

    let input_line = Line::from(vec![
        Span::styled(prefix, Style::default().fg(theme::FG_DIM)),
        Span::styled(display_text, Style::default().fg(theme::FG)),
        Span::styled(suffix, Style::default().fg(theme::FG_DIM)),
    ]);
    let input_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    frame.render_widget(Paragraph::new(input_line), input_area);

    // ボタンヒント
    let hints = Line::from(vec![
        Span::styled(" Enter ", theme::function_key_style()),
        Span::styled("決定 ", theme::function_bar_style()),
        Span::styled(" Esc ", theme::function_key_style()),
        Span::styled("キャンセル", theme::function_bar_style()),
    ]);
    let hint_area = Rect::new(inner.x, inner.y + 3, inner.width, 1);
    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        hint_area,
    );

    // カーソル位置（表示カラムベースで正確に配置）
    frame.set_cursor_position((
        inner.x + 1 + cursor_col_on_screen as u16,
        inner.y + 1,
    ));
}

/// メッセージダイアログ描画（git pull 結果等）
fn render_message(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    content: &str,
) {
    let lines: Vec<&str> = content.lines().collect();
    let content_height = lines.len().max(1) as u16;
    // タイトル(1) + 上余白(1) + 内容 + 下余白(1) + ヒント(1) + 枠(2)
    let dialog_height = (content_height + 5).min(area.height.saturating_sub(4));
    let dialog_width = 60u16.min(area.width.saturating_sub(4));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .title_style(theme::dialog_title_style())
        .border_style(theme::border_style(true));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let text_lines: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(theme::FG))))
        .collect();

    let max_text_height = inner.height.saturating_sub(2);
    let text_area = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), max_text_height);
    frame.render_widget(Paragraph::new(text_lines), text_area);

    let hints = Line::from(vec![
        Span::styled(" Esc ", theme::function_key_style()),
        Span::styled("閉じる", theme::function_bar_style()),
    ]);
    let hint_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1),
        inner.width,
        1,
    );
    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        hint_area,
    );
}

/// ブランチリストをキーワードでフィルタ（表示用・大文字小文字無視）
fn filter_branches_display(
    branches: &[(String, bool, String)],
    filter: &str,
) -> Vec<(String, bool, String)> {
    if filter.trim().is_empty() {
        return branches.to_vec();
    }
    let f = filter.to_lowercase();
    branches
        .iter()
        .filter(|(display, _, _)| display.to_lowercase().contains(&f))
        .cloned()
        .collect()
}

/// ブランチリストダイアログ描画
fn render_branch_list(
    frame: &mut Frame,
    area: Rect,
    branches: &[(String, bool, String)],
    filter_string: &str,
    search_input: &Option<String>,
    cursor: usize,
    scroll_offset: usize,
) {
    let filtered = filter_branches_display(branches, filter_string);
    // ターミナル横幅の75％をダイアログ幅に（最低幅は確保）
    let dialog_width = (area.width * 3 / 4).max(40).min(area.width.saturating_sub(4));

    let has_search_bar = search_input.is_some();
    let list_height = filtered.len().min(20) as u16;
    let extra = if has_search_bar { 1 } else { 0 };
    let dialog_height = (list_height + 4 + extra).min(area.height.saturating_sub(4));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    frame.render_widget(Clear, dialog_area);

    let title = if filter_string.is_empty() {
        format!(" Git Branch -a ({}) ", branches.len())
    } else {
        format!(" Git Branch ({}/{} 件) ", filtered.len(), branches.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(theme::dialog_title_style())
        .border_style(theme::border_style(true));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut row_y = inner.y;

    // 検索バー（search_input が Some のとき）
    if let Some(ref query) = search_input {
        let search_line = Line::from(vec![
            Span::styled("/ ", theme::function_key_style()),
            Span::styled(query.as_str(), Style::default().fg(theme::FG)),
        ]);
        let search_area = Rect::new(inner.x + 1, row_y, inner.width.saturating_sub(2), 1);
        frame.render_widget(Paragraph::new(search_line), search_area);
        row_y += 1;
    }

    let visible_rows = (inner.y + inner.height.saturating_sub(2)).saturating_sub(row_y) as usize;
    let effective_scroll = scroll_offset.min(filtered.len().saturating_sub(visible_rows));
    let end = (effective_scroll + visible_rows).min(filtered.len());

    for (i, (name, is_current, _)) in filtered[effective_scroll..end].iter().enumerate() {
        let list_idx = effective_scroll + i;
        let is_selected = list_idx == cursor;

        let prefix = if *is_current { "* " } else { "  " };

        let style = if is_selected && *is_current {
            Style::default().fg(theme::GREEN).bg(theme::BG_SELECTED).add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default().fg(theme::FG).bg(theme::BG_SELECTED).add_modifier(Modifier::BOLD)
        } else if *is_current {
            Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::FG)
        };

        let line = Line::from(Span::styled(
            format!("{}{}", prefix, name),
            style,
        ));
        let row_area = Rect::new(inner.x + 1, row_y + i as u16, inner.width.saturating_sub(2), 1);
        frame.render_widget(Paragraph::new(line), row_area);
    }

    // フッターヒント
    let hints = if search_input.is_some() {
        Line::from(vec![
            Span::styled(" Enter", theme::function_key_style()),
            Span::styled("検索適用 ", theme::function_bar_style()),
            Span::styled(" Esc", theme::function_key_style()),
            Span::styled("キャンセル", theme::function_bar_style()),
        ])
    } else {
        Line::from(vec![
            Span::styled(" /", theme::function_key_style()),
            Span::styled("検索 ", theme::function_bar_style()),
            Span::styled(" j/k", theme::function_key_style()),
            Span::styled("選択 ", theme::function_bar_style()),
            Span::styled(" Enter", theme::function_key_style()),
            Span::styled("チェックアウト ", theme::function_bar_style()),
            Span::styled(" Esc", theme::function_key_style()),
            Span::styled("閉じる", theme::function_bar_style()),
        ])
    };
    let hint_area = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1);
    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        hint_area,
    );
}

// ---------------------------------------------------------------------------
// 共通ヘルパー
// ---------------------------------------------------------------------------

/// 親領域の中央に配置された Rect を計算
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}
