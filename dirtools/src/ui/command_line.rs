// ui/command_line.rs - コマンド入力行描画
//
// コマンドモード時にファンクションバーの位置に表示される `:` プロンプト行。
// vi エディタのコマンドラインと同じ見た目・操作感を提供する。

use super::theme;
use crate::command::CommandState;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// 検索入力行を描画（vi の `/` プロンプト）
pub fn render_search(frame: &mut Frame, area: Rect, input: &str) {
    let w = area.width as usize;

    let prompt = Span::styled("/", theme::command_prompt_style());
    let input_text = Span::styled(
        input.to_string(),
        ratatui::style::Style::default().fg(theme::FG),
    );
    let cursor = Span::styled(
        "█",
        ratatui::style::Style::default().fg(theme::FG),
    );
    let back_label = "[戻る]";
    let back = Span::styled(
        back_label,
        ratatui::style::Style::default().fg(theme::FG_DIM),
    );

    let input_max = w.saturating_sub(1 + 1 + back_label.len() + 2);
    let input_display_len: usize = input.chars().count();
    let padding_len = input_max.saturating_sub(input_display_len);

    let line = Line::from(vec![
        prompt,
        input_text,
        cursor,
        Span::raw(" ".repeat(padding_len)),
        back,
        Span::raw(" "),
    ]);

    frame.render_widget(Paragraph::new(line), area);

    let cursor_x = area.x + 1 + input.len() as u16;
    let cursor_y = area.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}

/// コマンド入力行を描画
pub fn render(frame: &mut Frame, area: Rect, state: &CommandState) {
    let w = area.width as usize;

    // プロンプト ":"
    let prompt = Span::styled(":", theme::command_prompt_style());

    // 入力テキスト
    let input_text = Span::styled(
        state.input.clone(),
        ratatui::style::Style::default().fg(theme::FG),
    );

    // カーソル（ブロックカーソル表現）
    let cursor = Span::styled(
        "█",
        ratatui::style::Style::default().fg(theme::FG),
    );

    // [戻る] ボタン（右端）
    let back_label = "[戻る]";
    let back = Span::styled(
        back_label,
        ratatui::style::Style::default().fg(theme::FG_DIM),
    );

    // 入力部分の最大幅
    let input_max = w.saturating_sub(1 + 1 + back_label.len() + 2); // prompt + cursor + back + padding
    let padding_len = input_max.saturating_sub(state.input.len());

    let line = Line::from(vec![
        prompt,
        input_text,
        cursor,
        Span::raw(" ".repeat(padding_len)),
        back,
        Span::raw(" "),
    ]);

    frame.render_widget(Paragraph::new(line), area);

    // 実際のターミナルカーソル位置を設定
    let cursor_x = area.x + 1 + state.cursor_pos as u16;
    let cursor_y = area.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}
