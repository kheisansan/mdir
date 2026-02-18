// ui/header.rs - ヘッダーバー描画
//
// 画面最上部の1行。アプリ名・バージョンを表示する。
// Phase 3 でタブリスト・Git ブランチ表示を追加予定。

use super::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::Frame;

/// ヘッダーバーを描画
pub fn render(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" mdir", theme::header_style().fg(theme::CYAN)),
        Span::styled(" v0.1.0", theme::header_style().fg(theme::FG_DIM)),
    ]);

    // 背景塗り潰し
    let bg = ratatui::widgets::Block::default().style(theme::header_style());
    frame.render_widget(bg, area);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(line)
            .style(theme::header_style())
            .alignment(ratatui::layout::Alignment::Right),
        area,
    );
}
