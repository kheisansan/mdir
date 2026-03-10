// ui/welcome.rs - ウェルカムメッセージ描画
//
// 初回起動時にアプリの使い方を簡潔に案内する。
// 任意のキー押下またはクリックで消去される。

use super::theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// ウェルカムメッセージを画面中央に描画
pub fn render(frame: &mut Frame, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 14u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style(true));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let title_style = Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme::FG);
    let dim_style = Style::default().fg(theme::FG_DIM);

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled("mdir v0.1.1 へようこそ", title_style)),
        Line::raw(""),
        Line::from(Span::styled(
            "vi スタイルのターミナルファイルマネージャー",
            desc_style,
        )),
        Line::raw(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("h j k l  ", key_style),
            Span::styled("ナビゲーション（vi と同じ）", desc_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(":        ", key_style),
            Span::styled("コマンドモードに入る", desc_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(":help    ", key_style),
            Span::styled("詳細ヘルプを表示", desc_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("マウス   ", key_style),
            Span::styled("クリック・スクロール対応", desc_style),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "何かキーを押すと続行します...",
            dim_style,
        )),
    ];

    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        inner,
    );
}
