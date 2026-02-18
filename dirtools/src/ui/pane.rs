// ui/pane.rs - ファイルペイン描画
//
// パスバー + ファイル一覧を1ペイン分描画する。
// エントリ種別・マーク状態に応じたスタイル適用と仮想スクロールを行う。
//
// Unicode 安全な表示幅計算:
// - ファイル名の切り詰めは表示幅（カラム数）ベースで行う
// - CJK 文字（日本語等）は1文字=2カラム幅として正しく計算
// - マルチバイト UTF-8 文字の途中で切らないよう文字境界を保証

use super::theme;
use crate::fs::entry::EntryType;
use crate::fs::PaneState;
use crate::utils::format;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// パスバーを描画
pub fn render_path_bar(frame: &mut Frame, area: Rect, pane: &PaneState, is_active: bool) {
    let max_width = area.width.saturating_sub(4) as usize;
    let path_str = format::format_path(&pane.current_dir, max_width);

    let line = Line::from(vec![
        Span::styled(" \u{1F4C1} ", theme::path_style(is_active)), // 📁
        Span::styled(path_str, theme::path_style(is_active)),
    ]);

    let bg_style = if is_active {
        theme::header_style()
    } else {
        theme::header_style().fg(theme::FG_DIM)
    };

    frame.render_widget(
        Paragraph::new(line).style(bg_style),
        area,
    );
}

/// ファイルペインを描画
pub fn render_file_list(
    frame: &mut Frame,
    area: Rect,
    pane: &PaneState,
    is_active: bool,
    date_fmt: &str,
    time_fmt: &str,
) {
    let visible_rows = area.height as usize;

    // 各行を描画
    for row in 0..visible_rows {
        let entry_index = pane.scroll_offset + row;
        let y = area.y + row as u16;
        let line_area = Rect::new(area.x, y, area.width, 1);

        if entry_index >= pane.entries.len() {
            // エントリがない行は空白
            frame.render_widget(Paragraph::new(""), line_area);
            continue;
        }

        let entry = &pane.entries[entry_index];
        let is_cursor = entry_index == pane.cursor;
        let is_marked = pane.marked.contains(&entry.name);

        // 行の Span を構築
        let line = build_entry_line(entry, is_marked, area.width, date_fmt, time_fmt);

        // 行スタイル（カーソル行の背景色）
        let row_style = if is_cursor && is_active {
            theme::cursor_style()
        } else if is_cursor {
            theme::cursor_inactive_style()
        } else {
            ratatui::style::Style::default()
        };

        frame.render_widget(
            Paragraph::new(line).style(row_style),
            line_area,
        );
    }
}

/// 文字列を指定の表示幅（カラム数）に切り詰める
///
/// UTF-8 文字境界を保証し、マルチバイト文字の途中で切らない。
/// CJK 文字は1文字=2カラムとして正しく計算する。
/// 切り詰めた場合は末尾に `~` を付加する。
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let display_width = s.width();
    if display_width <= max_width {
        return s.to_string();
    }

    // 最大幅 - 1（'~' 用に1カラム確保）
    let target_width = max_width.saturating_sub(1);
    let mut current_width = 0;
    let mut result = String::new();

    for c in s.chars() {
        let char_width = c.width().unwrap_or(0);
        if current_width + char_width > target_width {
            break;
        }
        result.push(c);
        current_width += char_width;
    }

    result.push('~');
    result
}

/// 1エントリの表示行を構築
fn build_entry_line<'a>(
    entry: &crate::fs::entry::FileEntry,
    is_marked: bool,
    width: u16,
    date_fmt: &str,
    time_fmt: &str,
) -> Line<'a> {
    let w = width as usize;

    // マークインジケータ (2カラム)
    let mark = if is_marked { "● " } else { "  " };
    let mark_style = if is_marked {
        theme::marked_style()
    } else {
        ratatui::style::Style::default()
    };

    // ファイル名スタイル
    let name_style = if is_marked {
        theme::marked_style()
    } else {
        theme::entry_style(
            entry.is_dir(),
            entry.entry_type == EntryType::Executable,
            entry.is_symlink,
            entry.is_hidden,
        )
    };

    // サイズ (8カラム)
    let size_str = if entry.is_dir() {
        "   --".to_string()
    } else {
        format!("{:>8}", format::format_size(entry.size))
    };

    // 日時 (8カラム)
    let time_str = format!(
        "{:>8}",
        format::format_time(entry.modified, date_fmt, time_fmt)
    );

    // ファイル名の最大表示幅 = 全体幅 - mark(2) - size(8) - time(8) - padding(3)
    let name_max = w.saturating_sub(21);
    let mut name = entry.name.clone();
    if entry.is_dir() {
        name.push('/');
    }

    // 表示幅ベースで切り詰め（UTF-8 安全）
    let name = truncate_to_width(&name, name_max);

    // 名前の表示幅を計算し、残りをスペースで埋める
    let name_display_width = name.width();
    let padding = name_max.saturating_sub(name_display_width);

    Line::from(vec![
        Span::styled(mark.to_string(), mark_style),
        Span::styled(name, name_style),
        Span::raw(" ".repeat(padding)),
        Span::styled(size_str, ratatui::style::Style::default().fg(theme::FG_DIM)),
        Span::raw(" "),
        Span::styled(time_str, ratatui::style::Style::default().fg(theme::FG_DIM)),
    ])
}
