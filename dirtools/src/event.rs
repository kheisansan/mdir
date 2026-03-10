// event.rs - イベントハンドリング
//
// crossterm のキーボード/マウスイベントをポーリングし、
// 現在のモードに応じて適切な Action に変換する。
// Action パターンにより、このモジュールは状態を直接変更しない。
//
// === 日本語 IME 対策 ===
// macOS で日本語入力モード（ローマ字入力 / かな入力）がオンの場合、
// キーイベントがIMEに横取りされ、アルファベットの代わりに
// ひらがな等が送られてくることがある。
//
// 対策として以下を実装:
// 1. 矢印キー・Home/End/PageUp/PageDown 等の物理キーで全操作を可能に
//    （これらのキーは IME に影響されない）
// 2. JIS かな入力モードで各キー位置が生成するひらがなをマッピング
// 3. ローマ字入力モードで母音キーが即座に生成するひらがなをマッピング
// 4. 全角記号（：、。、〜 等）もマッピング

use crate::app::{Action, App, DialogState, PaneSide};
use crate::error::AppError;
use crate::mode::AppMode;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

/// イベントポーリングのタイムアウト
const POLL_TIMEOUT: Duration = Duration::from_millis(50);

/// ダブルクリック判定の最大間隔（ミリ秒）
const DOUBLE_CLICK_THRESHOLD_MS: u128 = 400;

/// 次のイベントを取得し Action に変換
pub fn next_action(app: &mut App) -> Result<Action, AppError> {
    if !event::poll(POLL_TIMEOUT)? {
        return Ok(Action::Noop);
    }

    let event = event::read()?;
    match event {
        // Windows では KeyPress / KeyRelease / KeyRepeat の3種類が飛んでくる。
        // Press のみ処理し、Release / Repeat は無視することで二重実行を防ぐ。
        // macOS/Linux では常に Press なのでこのフィルタは影響しない。
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(key, app),
        Event::Mouse(mouse) => handle_mouse(mouse, app),
        Event::Resize(_, _) => Ok(Action::Refresh),
        _ => Ok(Action::Noop),
    }
}

// ---------------------------------------------------------------------------
// JIS かな入力モードのキーマッピング
// ---------------------------------------------------------------------------
// JIS キーボードで「かな入力」がオンの場合、各キー位置が以下のひらがなを生成する:
//   q→た w→て e→い r→す t→か y→ん u→な i→に o→ら p→せ
//   a→ち s→と d→し f→は g→き h→く j→ま k→の l→り
//   z→つ x→さ c→そ v→ひ b→こ n→み m→も
//   .→る :→け ~→(へ)
//
// ローマ字入力モードでは母音キーのみ即座にひらがなを生成:
//   a→あ i→い u→う e→え o→お
// 子音キーはIMEがバッファリングするため、アプリに到達しない場合がある。
// その場合は矢印キー・ファンクションキー等の物理キー代替を使う。

/// かな文字から対応するASCIIキーのアクションへ変換（ノーマルモード用）
fn kana_to_normal_action(c: char) -> Option<Action> {
    Some(match c {
        // --- ナビゲーション ---
        'く' => Action::ParentDirectory,          // h
        'ま' => Action::MoveCursor(1),             // j
        'の' => Action::MoveCursor(-1),            // k
        'り' => Action::Enter,                     // l
        'き' => Action::JumpTop,                   // g
        'へ' | '〜' => Action::GoHome,             // ~ (かな: へ, 全角: 〜)

        // --- ファイル操作 ---
        'そ' => Action::CopyFiles,                // c
        'も' => Action::MoveFiles,                 // m
        'し' => Action::RequestDelete,             // d
        'す' => Action::RequestRename,             // r
        'み' => Action::RequestCreateDir,          // n
        'か' => Action::RequestCreateFile,         // t
        'さ' => Action::RequestChmod,              // x (かな: さ)

        // --- Git ---
        'ん' => Action::GitPull,                    // y (かな: ん)
        'な' => Action::GitCheckout,                // u (かな: な)
        'こ' => Action::GitBranchList,              // b (かな: こ)

        // --- 検索結果ナビゲーション ---
        'ら' => Action::FindPrev,                   // o (かな: ら)
        'せ' => Action::FindNext,                   // p (かな: せ)

        // --- マーキング ---
        'ち' | 'あ' => Action::ToggleMarkAll,      // a (かな: ち, ローマ字: あ)

        // --- 表示・ペイン ---
        'と' => Action::ShowSortMenu,              // s
        'つ' => Action::ToggleHidden,              // z (かな: つ)
        'る' | '。' => Action::GoHomeRight,        // . (かな: る, ローマ字: 。) 右ペインをホームへ
        'に' | 'い' => Action::ShowFileInfo,       // i (かな: に, ローマ字: い)

        // --- モード遷移 ---
        'け' | '：' => Action::EnterCommandMode,   // : (かな: け, 全角: ：)

        // --- アプリ制御 ---
        'た' => Action::Quit,                      // q

        _ => return None,
    })
}

/// かな文字から対応するASCIIキーのアクションへ変換（ヘルプモード用）
fn kana_to_help_action(c: char) -> Option<Action> {
    Some(match c {
        'た' => Action::ExitHelp,                  // q
        'ま' => Action::HelpScroll(1),             // j
        'の' => Action::HelpScroll(-1),            // k
        'し' => Action::HelpScroll(20),            // d
        'な' | 'う' => Action::HelpScroll(-20),    // u (かな: な, ローマ字: う)
        'き' => Action::HelpScroll(i32::MIN / 2),  // g
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// キーイベント: モード別にディスパッチ
// ---------------------------------------------------------------------------

fn handle_key(key: KeyEvent, app: &App) -> Result<Action, AppError> {
    // ウェルカム画面表示中は任意のキーで閉じる
    if app.show_welcome {
        return Ok(Action::DismissWelcome);
    }

    match app.mode {
        AppMode::Normal => handle_normal_key(key),
        AppMode::Command => handle_command_key(key),
        AppMode::Help => handle_help_key(key),
        AppMode::Dialog => handle_dialog_key(key, app),
    }
}

/// ノーマルモードのキーハンドリング（vi スタイル + IME 対策）
///
/// IME がオンの状態でも操作できるよう、以下の代替キーを提供:
/// - 矢印キー: ↑↓←→ でナビゲーション
/// - Home/End: 先頭/末尾ジャンプ
/// - PageUp/PageDown: ページ単位スクロール
/// - ファンクションキー: F2/F5/F6/F8 でファイル操作
/// - かな文字: JIS かな入力時の各キー位置に対応
fn handle_normal_key(key: KeyEvent) -> Result<Action, AppError> {
    Ok(match key.code {
        // === ナビゲーション（vi キー + 矢印キー + IME 対策）===
        KeyCode::Char('h') | KeyCode::Backspace | KeyCode::Left => Action::ParentDirectory,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveCursor(1),
        KeyCode::Char('k') | KeyCode::Up => Action::MoveCursor(-1),
        KeyCode::Char('l') | KeyCode::Enter | KeyCode::Right => Action::Enter,
        KeyCode::Char('g') | KeyCode::Home => Action::JumpTop,
        KeyCode::Char('G') | KeyCode::End => Action::JumpBottom,
        KeyCode::PageUp => Action::MoveCursor(-20),
        KeyCode::PageDown => Action::MoveCursor(20),
        KeyCode::Char('~') => Action::GoHome,

        // === ペイン ===
        KeyCode::Tab | KeyCode::BackTab => Action::SwitchPane,
        KeyCode::Char(',') => Action::GoHomeLeft,   // 左ペインを cd ~ に
        KeyCode::Char('.') => Action::GoHomeRight,  // 右ペインを cd ~ に
        KeyCode::Char('f') => Action::SyncDirToOtherPane, // アクティブペインのディレクトリを非アクティブに反映

        // === ファイル操作（アルファベット + ファンクションキー）===
        KeyCode::Char('c') | KeyCode::F(5) => Action::CopyFiles,
        KeyCode::Char('m') | KeyCode::F(6) => Action::MoveFiles,
        KeyCode::Char('d') | KeyCode::F(8) | KeyCode::Delete => Action::RequestDelete,
        KeyCode::Char('r') | KeyCode::F(2) => Action::RequestRename,
        KeyCode::Char('n') | KeyCode::F(7) => Action::RequestCreateDir,
        KeyCode::Char('t') | KeyCode::F(4) => Action::RequestCreateFile,
        KeyCode::Char('x') => Action::RequestChmod,
        KeyCode::Char('y') => Action::GitPull,
        KeyCode::Char('u') => Action::GitCheckout,
        KeyCode::Char('b') => Action::GitBranchList,

        // === マーキング ===
        KeyCode::Char(' ') | KeyCode::Insert => Action::ToggleMark,
        KeyCode::Char('a') => Action::ToggleMarkAll,

        // === 検索結果ナビゲーション ===
        KeyCode::Char('o') => Action::FindPrev,
        KeyCode::Char('p') => Action::FindNext,

        // === 表示 ===
        KeyCode::Char('s') => Action::ShowSortMenu,
        KeyCode::Char('z') => Action::ToggleHidden,
        KeyCode::Char('i') => Action::ShowFileInfo,

        // === モード遷移 ===
        KeyCode::Char(':') => Action::EnterCommandMode,

        // === アプリ制御 ===
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,

        // === 日本語 IME 対策: かな文字・全角記号をマッピング ===
        KeyCode::Char(c) => {
            kana_to_normal_action(c).unwrap_or(Action::Noop)
        }

        _ => Action::Noop,
    })
}

/// コマンドモードのキーハンドリング
/// （コマンド入力中は IME で日本語テキストを入力する場面もあるため、
///   かなマッピングは適用せず、全文字をそのまま入力として受け付ける）
fn handle_command_key(key: KeyEvent) -> Result<Action, AppError> {
    Ok(match key.code {
        KeyCode::Esc => Action::ExitCommandMode,
        KeyCode::Enter => Action::CommandExecute,
        KeyCode::Backspace => Action::CommandBackspace,
        KeyCode::Tab => Action::CommandComplete,
        KeyCode::Up => Action::CommandHistoryPrev,
        KeyCode::Down => Action::CommandHistoryNext,
        KeyCode::Char(c) => Action::CommandInput(c),
        _ => Action::Noop,
    })
}

/// ヘルプモードのキーハンドリング（IME 対策込み）
fn handle_help_key(key: KeyEvent) -> Result<Action, AppError> {
    Ok(match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::ExitHelp,
        KeyCode::Char('j') | KeyCode::Down => Action::HelpScroll(1),
        KeyCode::Char('k') | KeyCode::Up => Action::HelpScroll(-1),
        KeyCode::Char('d') | KeyCode::PageDown => Action::HelpScroll(20),
        KeyCode::Char('u') | KeyCode::PageUp => Action::HelpScroll(-20),
        KeyCode::Char('g') | KeyCode::Home => Action::HelpScroll(i32::MIN / 2),
        KeyCode::Char('G') | KeyCode::End => Action::HelpScroll(i32::MAX / 2),

        // === 日本語 IME 対策: かな文字をマッピング ===
        KeyCode::Char(c) => {
            kana_to_help_action(c).unwrap_or(Action::Noop)
        }

        _ => Action::Noop,
    })
}

/// ダイアログモードのキーハンドリング
fn handle_dialog_key(key: KeyEvent, app: &App) -> Result<Action, AppError> {
    Ok(match &app.dialog {
        Some(DialogState::Confirm { focus_yes, .. }) => match key.code {
            // y/n に加え、左右矢印キーでの選択も可能に
            KeyCode::Char('y') => Action::DialogConfirm,
            KeyCode::Char('n') | KeyCode::Esc => Action::DialogCancel,
            KeyCode::Enter => {
                if *focus_yes {
                    Action::DialogConfirm
                } else {
                    Action::DialogCancel
                }
            }
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                // フォーカストグル（この場でやるには Action が要るが、簡略化）
                Action::Noop
            }
            _ => Action::Noop,
        },
        Some(DialogState::Input { .. }) => match key.code {
            KeyCode::Enter => Action::DialogConfirm,
            KeyCode::Esc => Action::DialogCancel,
            KeyCode::Backspace => Action::DialogBackspace,
            KeyCode::Left => Action::DialogCursorLeft,
            KeyCode::Right => Action::DialogCursorRight,
            KeyCode::Home => Action::DialogHome,
            KeyCode::End => Action::DialogEnd,
            KeyCode::Char(c) => Action::DialogInput(c),
            _ => Action::Noop,
        },
        Some(DialogState::Message { .. }) => match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => Action::DialogCancel,
            _ => Action::Noop,
        },
        Some(DialogState::CopyMoveConflict { .. }) => match key.code {
            KeyCode::Tab | KeyCode::Right => Action::DialogConflictNext,
            KeyCode::Left => Action::DialogConflictPrev,
            KeyCode::Enter => Action::DialogConfirm,
            KeyCode::Esc | KeyCode::Char('n') => Action::DialogCancel,
            KeyCode::Char('y') => Action::DialogConfirm, // 現在の focus で確定（0=上書き 1=リネーム 2=キャンセル）
            _ => Action::Noop,
        },
        Some(DialogState::BranchList { search_input, .. }) => {
            if search_input.is_some() {
                match key.code {
                    KeyCode::Enter => Action::BranchListSearchEnter,
                    KeyCode::Esc => Action::BranchListSearchCancel,
                    KeyCode::Backspace => Action::BranchListSearchBackspace,
                    KeyCode::Char(c) => Action::BranchListSearchInput(c),
                    _ => Action::Noop,
                }
            } else {
                match key.code {
                    KeyCode::Char('/') => Action::BranchListStartSearch,
                    KeyCode::Char('j') | KeyCode::Down => Action::BranchListMoveCursor(1),
                    KeyCode::Char('k') | KeyCode::Up => Action::BranchListMoveCursor(-1),
                    KeyCode::Char('g') | KeyCode::Home => Action::BranchListMoveCursor(i32::MIN / 2),
                    KeyCode::Char('G') | KeyCode::End => Action::BranchListMoveCursor(i32::MAX / 2),
                    KeyCode::PageDown => Action::BranchListMoveCursor(10),
                    KeyCode::PageUp => Action::BranchListMoveCursor(-10),
                    KeyCode::Enter => Action::DialogConfirm,
                    KeyCode::Esc | KeyCode::Char('q') => Action::DialogCancel,
                    _ => Action::Noop,
                }
            }
        }
        None => Action::Noop,
    })
}

// ---------------------------------------------------------------------------
// マウスイベント
// ---------------------------------------------------------------------------

fn handle_mouse(mouse: MouseEvent, app: &mut App) -> Result<Action, AppError> {
    if app.show_welcome {
        return Ok(Action::DismissWelcome);
    }

    // ダイアログモードでのマウスイベント特殊処理
    if app.mode == AppMode::Dialog {
        return Ok(match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                handle_left_click(mouse.column, mouse.row, app)
            }
            MouseEventKind::ScrollUp => {
                if matches!(app.dialog, Some(DialogState::BranchList { .. })) {
                    Action::BranchListMoveCursor(-3)
                } else {
                    Action::Noop
                }
            }
            MouseEventKind::ScrollDown => {
                if matches!(app.dialog, Some(DialogState::BranchList { .. })) {
                    Action::BranchListMoveCursor(3)
                } else {
                    Action::Noop
                }
            }
            _ => Action::Noop,
        });
    }

    Ok(match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(mouse.column, mouse.row, app)
        }
        MouseEventKind::Down(MouseButton::Right) => {
            Action::Noop
        }
        MouseEventKind::ScrollUp => Action::MoveCursor(-3),
        MouseEventKind::ScrollDown => Action::MoveCursor(3),
        _ => Action::Noop,
    })
}

/// ダブルクリック判定：直前のクリックと同じ位置で DOUBLE_CLICK_THRESHOLD_MS 以内か
fn is_double_click(app: &App, x: u16, y: u16) -> bool {
    if let Some((prev_x, prev_y, prev_time)) = app.last_click {
        prev_x == x && prev_y == y && prev_time.elapsed().as_millis() < DOUBLE_CLICK_THRESHOLD_MS
    } else {
        false
    }
}

/// 左クリック処理: 位置に応じたアクション決定
fn handle_left_click(x: u16, y: u16, app: &mut App) -> Action {
    let (w, h) = app.terminal_size;

    // --- ヘルプモード ---
    // ヘルプ画面表示中はクリックで閉じる
    if app.mode == AppMode::Help {
        return Action::ExitHelp;
    }

    // --- ダイアログモード ---
    // ダイアログ表示中のクリック処理
    if app.mode == AppMode::Dialog {
        return handle_dialog_click(x, y, app);
    }

    // --- コマンドモードの [戻る] ボタン判定 ---
    if app.mode == AppMode::Command && y == h.saturating_sub(1) {
        // 右端付近 "[戻る]" 部分（8文字分の余裕）
        if x >= w.saturating_sub(10) {
            return Action::ExitCommandMode;
        }
    }

    // --- ファンクションバー行のクリック判定 ---
    if y == h.saturating_sub(1) && app.mode == AppMode::Normal {
        let has_git = app.active_pane_ref().git_branch.is_some();
        return handle_function_bar_click(x, w, app.find_state.is_some(), has_git);
    }

    // --- ファイルペイン領域のクリック判定（y >= 2 && y < h-2）---
    if y >= 2 && y < h.saturating_sub(2) {
        let double = is_double_click(app, x, y);
        // クリック情報を記録（ダブルクリック検出用）
        app.last_click = Some((x, y, Instant::now()));

        let border_x = (w as f32 * app.pane_ratio) as u16;
        let side = if x < border_x {
            PaneSide::Left
        } else {
            PaneSide::Right
        };

        let pane = match side {
            PaneSide::Left => &app.left_pane,
            PaneSide::Right => &app.right_pane,
        };

        // クリックされたエントリのインデックスを計算
        let clicked_index = pane.scroll_offset + (y as usize).saturating_sub(2);

        if clicked_index < pane.entries.len() {
            // ダブルクリック → ディレクトリに入る / ファイルを開く
            if double && side == app.active_pane {
                return Action::Enter;
            }

            // ペイン切り替え（必要なら）+ カーソル移動
            if side != app.active_pane {
                return Action::ActivatePane(side);
            }

            // 同じペイン内でのクリック → カーソルを移動
            let delta = clicked_index as i32 - pane.cursor as i32;
            return Action::MoveCursor(delta);
        }

        // ペイン領域内のクリック（エントリがない部分）→ ペインは切り替え
        if side != app.active_pane {
            return Action::ActivatePane(side);
        }
    }

    // --- パスバー行（y == 1）のクリック → ペイン切り替え ---
    if y == 1 {
        let border_x = (w as f32 * app.pane_ratio) as u16;
        let side = if x < border_x {
            PaneSide::Left
        } else {
            PaneSide::Right
        };
        if side != app.active_pane {
            return Action::ActivatePane(side);
        }
    }

    Action::Noop
}

/// ファンクションバーのクリック判定
///
/// ボタン配置（表示カラム位置）: [:]コマンド 削除後
/// " [F2/r]名前変更 [F5/c]コピー [F6/m]移動 [F8/d]削除 [:help]ヘルプ [q/Esc]終了 "
///   1..16          16..29       29..40      40..51      51..65       65..77
/// 検索時追加:
///   [o]前候補 [p]次候補
///   77..87    87..
fn handle_function_bar_click(x: u16, _w: u16, has_find: bool, has_git: bool) -> Action {
    let x = x as usize;

    if (1..16).contains(&x) {
        Action::RequestRename
    } else if (16..29).contains(&x) {
        Action::CopyFiles
    } else if (29..40).contains(&x) {
        Action::MoveFiles
    } else if (40..51).contains(&x) {
        Action::RequestDelete
    } else if (51..65).contains(&x) {
        Action::EnterHelp(None)
    } else if (65..77).contains(&x) {
        Action::Quit
    } else if has_find && (77..87).contains(&x) {
        Action::FindPrev
    } else if has_find && x >= 87 && (!has_git || x < 97) {
        Action::FindNext
    } else if has_git {
        let git_start = if has_find { 97 } else { 77 };
        let git_pull_end = git_start + 12;
        let git_checkout_end = git_pull_end + 16;
        if x >= git_start && x < git_pull_end {
            Action::GitPull
        } else if x >= git_pull_end && x < git_checkout_end {
            Action::GitCheckout
        } else if x >= git_checkout_end {
            Action::GitBranchList
        } else {
            Action::Noop
        }
    } else {
        Action::Noop
    }
}

/// ダイアログ内のクリック処理
fn handle_dialog_click(x: u16, y: u16, app: &mut App) -> Action {
    let (w, h) = app.terminal_size;

    // ブランチリストダイアログの場合は専用処理
    if let Some(DialogState::BranchList { branches, filter_string, search_input, cursor: _, scroll_offset }) = &app.dialog {
        let filtered_len = if filter_string.trim().is_empty() {
            branches.len()
        } else {
            let f = filter_string.to_lowercase();
            branches.iter().filter(|(d, _, _)| d.to_lowercase().contains(&f)).count()
        };
        let dialog_width = (w * 3 / 4).max(40).min(w.saturating_sub(4));
        let list_height = filtered_len.min(20) as u16;
        let extra = if search_input.is_some() { 1 } else { 0 };
        let dialog_height = (list_height + 4 + extra).min(h.saturating_sub(4));
        let dialog_x = (w.saturating_sub(dialog_width)) / 2;
        let dialog_y = (h.saturating_sub(dialog_height)) / 2;

        if x < dialog_x || x >= dialog_x + dialog_width || y < dialog_y || y >= dialog_y + dialog_height {
            return Action::DialogCancel;
        }

        let mut inner_y = dialog_y + 1;
        if search_input.is_some() {
            inner_y += 1;
        }
        let inner_end_y = dialog_y + dialog_height - 2;
        if y >= inner_y && y < inner_end_y {
            let clicked_row = (y - inner_y) as usize;
            let clicked_idx = *scroll_offset + clicked_row;
            if clicked_idx < filtered_len {
                let double = is_double_click(app, x, y);
                app.last_click = Some((x, y, Instant::now()));
                if double {
                    if let Some(DialogState::BranchList { cursor, .. }) = &mut app.dialog {
                        *cursor = clicked_idx;
                    }
                    return Action::DialogConfirm;
                }
                if let Some(DialogState::BranchList { cursor, .. }) = &mut app.dialog {
                    *cursor = clicked_idx;
                }
                return Action::Noop;
            }
        }
        return Action::Noop;
    }

    // 通常ダイアログ（Confirm/Input/Message）
    let dialog_width = 44u16.min(w.saturating_sub(4));
    let dialog_height = 7u16;
    let dialog_x = (w.saturating_sub(dialog_width)) / 2;
    let dialog_y = (h.saturating_sub(dialog_height)) / 2;

    if x < dialog_x || x >= dialog_x + dialog_width || y < dialog_y || y >= dialog_y + dialog_height {
        return Action::DialogCancel;
    }

    if let Some(DialogState::Confirm { .. }) = &app.dialog {
        let btn_y = dialog_y + 4;
        if y == btn_y {
            let center = dialog_x + dialog_width / 2;
            if x < center {
                return Action::DialogConfirm;
            } else {
                return Action::DialogCancel;
            }
        }
    }

    Action::Noop
}
