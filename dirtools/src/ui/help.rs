// ui/help.rs - :help ヘルプ画面描画
//
// 全画面オーバーレイとして操作方法・キーバインド・コマンド一覧を表示する。
// j/k でスクロール可能。`:help <topic>` でトピック別表示にも対応。

use super::theme;
use crate::app::HelpState;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// ヘルプテキストの全行を生成
fn build_help_lines() -> Vec<Line<'static>> {
    let section_style = Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(theme::FG).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(theme::FG);
    let dim = Style::default().fg(theme::FG_DIM);

    // ヘルパー: キーバインド行を生成
    let kb = |key: &'static str, desc: &'static str| -> Line<'static> {
        Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{:<14}", key), key_style),
            Span::styled(desc, desc_style),
        ])
    };

    vec![
        Line::raw(""),
        Line::from(Span::styled("━━━ ナビゲーション（vi スタイル）━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        kb("h / ← / BS", "親ディレクトリへ移動"),
        kb("j / ↓", "カーソルを下に移動"),
        kb("k / ↑", "カーソルを上に移動"),
        kb("l / → / Enter", "ディレクトリに入る / ファイルを開く"),
        kb("g / Home", "先頭へジャンプ"),
        kb("G / End", "末尾へジャンプ"),
        kb("PgUp / PgDn", "ページ単位スクロール"),
        kb("~", "ホームディレクトリへ移動"),
        kb("Tab", "アクティブペインを切り替え"),
        kb("o", "検索結果：前の候補へ（:find 実行後）"),
        kb("p", "検索結果：次の候補へ（:find 実行後）"),
        Line::raw(""),
        Line::from(Span::styled("━━━ ファイル操作 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        kb("c / F5", "対向ペインへコピー"),
        kb("m / F6", "対向ペインへ移動"),
        kb("d / F8 / Del", "削除（ゴミ箱へ移動）"),
        kb("r / F2", "名前変更"),
        kb("n / F7", "新規フォルダ作成"),
        kb("t / F4", "新規ファイル作成"),
        kb("x", "パーミッション変更（chmod, Unix のみ）"),
        kb("Space / Ins", "マーク / マーク解除"),
        kb("a", "全選択 / 全解除"),
        kb("s", "ソートメニュー"),
        kb(".", "隠しファイル表示切り替え"),
        kb("i", "ファイル情報"),
        kb("y", "Git Pull（Git リポジトリ内のみ）"),
        Line::raw(""),
        Line::from(Span::styled("━━━ コマンドモード ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("「:」を入力するとコマンドモードに入ります（vi と同じ）", dim),
        ]),
        Line::raw(""),
        kb(":help [topic]", "このヘルプ画面を表示"),
        kb(":q / :quit", "アプリケーションを終了"),
        kb(":cd <パス>", "ディレクトリ移動"),
        kb(":sort <基準>", "ソート（name/size/date/ext）"),
        kb(":set <設定>", "設定変更"),
        kb(":!<コマンド>", "シェルコマンド実行"),
        kb(":mkdir <名前>", "ディレクトリ作成"),
        kb(":touch <名前>", "ファイル作成"),
        kb(":find <文字列>", "ファイル名検索（o/pで候補移動）"),
        kb(":bookmark [n]", "ブックマーク追加"),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("Esc キーまたは [戻る] クリックでコマンドモードを終了します。", dim),
        ]),
        Line::raw(""),
        Line::from(Span::styled("━━━ マウス操作 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        kb("左クリック", "ファイル選択 / ペイン切り替え"),
        kb("ダブルクリック", "ディレクトリに入る / ファイルを開く"),
        kb("スクロール", "ファイル一覧のスクロール"),
        Line::raw(""),
        Line::from(Span::styled("━━━ 日本語入力モード対応 ━━━━━━━━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("日本語 IME がオンの状態でも操作できます。", desc_style),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("・矢印キー / F キー / Home / End は IME に影響されません", dim),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("・JIS かな入力時のひらがなにも対応しています", dim),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("・ローマ字入力の場合は矢印キー等の物理キーをお使いください", dim),
        ]),
        Line::raw(""),
        Line::from(Span::styled("━━━ 設定 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", section_style)),
        Line::raw(""),
        Line::from(vec![
            Span::raw("    "),
            Span::styled("設定ファイル: ", key_style),
            Span::styled("~/.config/mdir/config.toml", desc_style),
        ]),
        Line::raw(""),
    ]
}

/// ヘルプ画面を全画面オーバーレイで描画
pub fn render(frame: &mut Frame, area: Rect, state: &HelpState) {
    // 背景クリア
    frame.render_widget(Clear, area);

    // 枠
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" mdir v0.1.0 - ヘルプ ")
        .title_alignment(Alignment::Center)
        .title_style(theme::dialog_title_style())
        .border_style(theme::border_style(true));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // ヘルプテキスト
    let lines = build_help_lines();
    let total_lines = lines.len();

    // スクロール適用
    let max_scroll = total_lines.saturating_sub(inner.height as usize);
    let scroll = state.scroll_offset.min(max_scroll);

    let content = Paragraph::new(lines).scroll((scroll as u16, 0));
    let content_area = Rect::new(inner.x, inner.y, inner.width, inner.height.saturating_sub(1));
    frame.render_widget(content, content_area);

    // フッター
    let footer = Line::from(vec![
        Span::styled(" j/k", theme::function_key_style()),
        Span::styled(":スクロール ", theme::function_bar_style()),
        Span::styled(" q/Esc", theme::function_key_style()),
        Span::styled(":閉じる", theme::function_bar_style()),
        Span::raw("  "),
        Span::styled(
            format!("{}/{} 行", scroll + 1, total_lines),
            Style::default().fg(theme::FG_DIM),
        ),
    ]);
    let footer_area = Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1);
    frame.render_widget(
        Paragraph::new(footer).alignment(Alignment::Right),
        footer_area,
    );
}
