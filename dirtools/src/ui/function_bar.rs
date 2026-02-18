// ui/function_bar.rs - ファンクションバー描画
//
// 画面最下行。主要操作のショートカットキーを常時表示する。
// マウスクリック対応の視覚的ボタンとして機能する。
// 検索結果がある場合は o/p ナビゲーションボタンも表示する。

use super::theme;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// ファンクションバーに表示するボタン定義（ファンクションキー/一般キー 併記）
const BUTTONS: &[(&str, &str)] = &[
    ("F2/r", "名前変更"),
    ("F5/c", "コピー"),
    ("F6/m", "移動"),
    ("F8/d", "削除"),
    (":", "コマンド"),
    (":help", "ヘルプ"),
    ("q/Esc", "終了"),
];

/// 検索結果がある場合に追加表示するボタン
const FIND_BUTTONS: &[(&str, &str)] = &[
    ("o", "前候補"),
    ("p", "次候補"),
];

/// Git リポジトリ内の場合に追加表示するボタン
const GIT_BUTTONS: &[(&str, &str)] = &[
    ("y", "Git Pull"),
];

/// ファンクションバーを描画
///
/// `has_find` が true の場合、検索結果ナビゲーション用の o/p ボタンも表示する。
/// `has_git` が true の場合、Git 操作用のボタンも表示する。
pub fn render(frame: &mut Frame, area: Rect, has_find: bool, has_git: bool) {
    let mut spans = Vec::new();
    spans.push(Span::raw(" "));

    for (key, label) in BUTTONS {
        spans.push(Span::styled(
            format!("[{}]", key),
            theme::function_key_style(),
        ));
        spans.push(Span::styled(
            format!("{} ", label),
            theme::function_bar_style(),
        ));
    }

    // 検索結果がある場合は o/p ボタンを追加
    if has_find {
        for (key, label) in FIND_BUTTONS {
            spans.push(Span::styled(
                format!("[{}]", key),
                theme::function_key_style(),
            ));
            spans.push(Span::styled(
                format!("{} ", label),
                theme::function_bar_style(),
            ));
        }
    }

    // Git リポジトリ内の場合は Git ボタンを追加
    if has_git {
        for (key, label) in GIT_BUTTONS {
            spans.push(Span::styled(
                format!("[{}]", key),
                theme::function_key_style(),
            ));
            spans.push(Span::styled(
                format!("{} ", label),
                theme::function_bar_style(),
            ));
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(theme::function_bar_style()),
        area,
    );
}
