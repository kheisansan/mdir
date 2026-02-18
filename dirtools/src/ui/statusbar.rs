// ui/statusbar.rs - ステータスバー描画
//
// 画面下部の情報行。カーソル位置のファイル情報またはマーク選択サマリーを表示する。
// 一時メッセージ（成功/エラー等）がある場合はそちらを優先表示する。

use super::theme;
use crate::app::{App, MessageLevel};
use crate::utils::format;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// ステータスバーを描画
pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // 一時メッセージが有効ならそちらを表示
    if let Some(ref msg) = app.message {
        if !msg.is_expired() {
            let style = match msg.level {
                MessageLevel::Error => theme::error_style(),
                MessageLevel::Success => theme::success_style(),
                MessageLevel::Warning => theme::error_style().fg(theme::ORANGE),
                MessageLevel::Info => theme::info_style(),
            };
            let line = Line::from(Span::styled(format!(" {}", msg.text), style));
            frame.render_widget(
                Paragraph::new(line).style(theme::status_style()),
                area,
            );
            return;
        }
    }

    let pane = app.active_pane_ref();

    // マーク選択サマリー
    if let Some(summary) = pane.mark_summary() {
        let line = Line::from(Span::styled(
            format!(" {}", summary),
            theme::info_style(),
        ));
        frame.render_widget(
            Paragraph::new(line).style(theme::status_style()),
            area,
        );
        return;
    }

    // カーソル位置のファイル情報
    if let Some(entry) = pane.current_entry() {
        let mut parts = vec![
            Span::styled(format!(" {}", entry.name), ratatui::style::Style::default().fg(theme::FG)),
            Span::styled(" │ ", ratatui::style::Style::default().fg(theme::FG_DIM)),
            Span::styled(
                format::format_size(entry.size),
                ratatui::style::Style::default().fg(theme::FG),
            ),
            Span::styled(" │ ", ratatui::style::Style::default().fg(theme::FG_DIM)),
            Span::styled(
                format::format_time(
                    entry.modified,
                    "%Y-%m-%d %H:%M",
                    "%Y-%m-%d %H:%M",
                ),
                ratatui::style::Style::default().fg(theme::FG),
            ),
            Span::styled(" │ ", ratatui::style::Style::default().fg(theme::FG_DIM)),
            Span::styled(
                entry.permission_string(),
                ratatui::style::Style::default().fg(theme::FG),
            ),
        ];

        // Git ブランチ表示
        if let Some(ref branch) = pane.git_branch {
            parts.push(Span::styled(" │ ", ratatui::style::Style::default().fg(theme::FG_DIM)));
            parts.push(Span::styled(
                format!("\u{E0A0} {}", branch),
                ratatui::style::Style::default().fg(theme::GREEN),
            ));
        }

        let line = Line::from(parts);
        frame.render_widget(
            Paragraph::new(line).style(theme::status_style()),
            area,
        );
    } else {
        // 空ディレクトリ
        let line = Line::from(Span::styled(" （空）", theme::status_style().fg(theme::FG_DIM)));
        frame.render_widget(
            Paragraph::new(line).style(theme::status_style()),
            area,
        );
    }
}
