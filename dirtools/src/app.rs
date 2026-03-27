// app.rs - アプリケーション状態管理
//
// アプリ全体のルート状態 `App` と、イベントから生成される `Action` を定義する。
// Action パターン: イベントハンドラは直接状態を変更せず Action を返し、
// App::apply_action() で一元的に状態を更新する。

use crate::command::{self, CommandState};
use crate::config::Config;
use crate::error::AppError;
use crate::fs::entry::SortCriteria;
use crate::fs::operations::{self, ConflictResolution};
use crate::fs::PaneState;
use crate::mode::AppMode;
use std::path::PathBuf;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Action - イベントハンドラが返す状態変更指示
// ---------------------------------------------------------------------------

/// アプリケーション状態を変更するアクション
#[allow(dead_code)]
pub enum Action {
    // --- ナビゲーション ---
    MoveCursor(i32),
    Enter,
    ParentDirectory,
    GoHome,
    GoHomeLeft,   // 左ペインをホームに
    GoHomeRight,  // 右ペインをホームに
    JumpTop,
    JumpBottom,

    // --- ペイン ---
    SwitchPane,
    ActivatePane(PaneSide),
    SetPaneRatio(f32),
    /// アクティブペインのカレントディレクトリを非アクティブペインに反映
    SyncDirToOtherPane,

    // --- ファイル操作 ---
    CopyFiles,
    MoveFiles,
    RequestDelete,
    ExecuteDelete,
    RequestRename,
    ExecuteRename(String),
    RequestCreateDir,
    ExecuteCreateDir(String),
    RequestCreateFile,
    ExecuteCreateFile(String),
    RequestChmod,

    // --- マーキング ---
    ToggleMark,
    ToggleMarkAll,

    // --- 表示 ---
    ToggleHidden,
    ShowSortMenu,
    SetSort(SortCriteria),
    ShowFileInfo,
    Refresh,

    // --- モード遷移 ---
    EnterCommandMode,
    ExitCommandMode,
    EnterHelp(Option<String>),
    ExitHelp,

    // --- コマンドモード操作 ---
    CommandInput(char),
    CommandBackspace,
    CommandExecute,
    CommandComplete,
    CommandHistoryPrev,
    CommandHistoryNext,

    // --- ヘルプ ---
    HelpScroll(i32),

    // --- ダイアログ ---
    ShowConfirmDialog {
        title: String,
        message: String,
    },
    DialogConfirm,
    DialogCancel,
    DialogInput(char),
    /// 衝突ダイアログの選択を次へ（Tab/Right）
    DialogConflictNext,
    /// 衝突ダイアログの選択を前へ（Left）
    DialogConflictPrev,
    DialogBackspace,
    DialogCursorLeft,
    DialogCursorRight,
    DialogHome,
    DialogEnd,

    // --- Git ---
    GitPull,
    GitCheckout,
    GitBranchList,
    BranchListMoveCursor(i32),
    BranchListStartSearch,
    BranchListSearchInput(char),
    BranchListSearchBackspace,
    BranchListSearchEnter,
    BranchListSearchCancel,

    // --- 検索 ---
    FindNext,
    FindPrev,
    EnterSearchMode,
    SearchInput(char),
    SearchBackspace,
    SearchExecute,
    ExitSearchMode,
    /// 検索クエリをクリア
    ClearSearch,

    // --- クリップボード ---
    CopyFileName,
    CopyFullPath,

    // --- アプリ制御 ---
    Quit,
    ShowMessage(String, MessageLevel),
    DismissWelcome,
    Noop,
}

/// ペイン左右
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

/// メッセージレベル
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MessageLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// 一時メッセージ
pub struct TimedMessage {
    pub text: String,
    pub level: MessageLevel,
    pub created_at: Instant,
}

impl TimedMessage {
    pub fn new(text: String, level: MessageLevel) -> Self {
        Self {
            text,
            level,
            created_at: Instant::now(),
        }
    }

    /// 表示期限切れかどうか（2秒）
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= 2
    }
}

/// ダイアログの種類と状態
pub enum DialogState {
    /// 確認ダイアログ（削除等）
    Confirm {
        title: String,
        message: String,
        focus_yes: bool,
        pending: PendingAction,
    },
    /// 入力ダイアログ（リネーム、新規作成等）
    Input {
        title: String,
        value: String,
        cursor_pos: usize,
        pending: PendingInputAction,
    },
    /// メッセージダイアログ（git pull 結果等、読み取り専用）
    Message {
        title: String,
        content: String,
    },
    /// ブランチリストダイアログ（git branch -a + キーワード検索）
    BranchList {
        /// (表示名, 現在ブランチか, checkout 引数)
        branches: Vec<(String, bool, String)>,
        filter_string: String,
        search_input: Option<String>,
        cursor: usize,
        scroll_offset: usize,
    },
    /// コピー/移動時の同名ファイル衝突（上書き or リネーム or キャンセル）
    CopyMoveConflict {
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        index: usize,
        is_move: bool,
        /// 0=上書き 1=リネームしてコピー/移動 2=キャンセル
        focus: usize,
    },
}

/// 確認ダイアログ確定後のアクション
pub enum PendingAction {
    Delete(Vec<PathBuf>),
}

/// 入力ダイアログ確定後のアクション
pub enum PendingInputAction {
    Rename(PathBuf),
    CreateDir,
    CreateFile,
    Chmod(Vec<PathBuf>),
    GitCheckout,
}

/// ヘルプ画面の状態
#[derive(Default)]
#[allow(dead_code)]
pub struct HelpState {
    pub scroll_offset: usize,
    pub topic: Option<String>,
}

/// ファイル検索の状態（`:find` コマンド用）
#[allow(dead_code)]
pub struct FindState {
    /// 検索文字列
    pub query: String,
    /// マッチしたエントリのインデックス一覧（entries 内の位置）
    pub matches: Vec<usize>,
    /// 現在のマッチ位置（matches 内のインデックス）
    pub current: usize,
}

// ---------------------------------------------------------------------------
// App - アプリケーション状態ルート
// ---------------------------------------------------------------------------

/// アプリケーション全体の状態
pub struct App {
    pub mode: AppMode,
    pub left_pane: PaneState,
    pub right_pane: PaneState,
    pub active_pane: PaneSide,
    pub command_state: CommandState,
    pub dialog: Option<DialogState>,
    pub help_state: HelpState,
    pub pane_ratio: f32,
    pub config: Config,
    pub message: Option<TimedMessage>,
    pub should_quit: bool,
    pub show_welcome: bool,
    /// ターミナルサイズ
    pub terminal_size: (u16, u16),
    /// 直前のクリック情報（ダブルクリック検出用: x, y, 時刻）
    pub last_click: Option<(u16, u16, Instant)>,
    /// ファイル検索状態（`:find` / `/` 検索の結果）
    pub find_state: Option<FindState>,
    /// 永続検索クエリ（ペイン切替・ディレクトリ移動後も維持）
    pub search_query: Option<String>,
    /// 検索モード（`/`）の入力バッファ
    pub search_input: String,
    /// 自動リフレッシュ用タイマー
    last_auto_refresh: Instant,
}

impl App {
    /// 新規作成
    pub fn new(
        config: Config,
        left_path: PathBuf,
        right_path: PathBuf,
    ) -> Result<Self, AppError> {
        let show_hidden = config.general.show_hidden;
        let pane_ratio = config.ui.pane_ratio;
        let show_welcome = config.ui.show_welcome && Config::is_first_launch();

        let history = command::history::CommandHistory::load(&Config::config_dir());
        let command_state = CommandState::new(history);

        Ok(Self {
            mode: AppMode::Normal,
            left_pane: PaneState::new(left_path, show_hidden)?,
            right_pane: PaneState::new(right_path, show_hidden)?,
            active_pane: PaneSide::Left,
            command_state,
            dialog: None,
            help_state: HelpState::default(),
            pane_ratio,
            config,
            message: None,
            should_quit: false,
            show_welcome,
            terminal_size: (80, 24),
            last_click: None,
            find_state: None,
            search_query: None,
            search_input: String::new(),
            last_auto_refresh: Instant::now(),
        })
    }

    /// アクティブペインの可変参照
    pub fn active_pane_mut(&mut self) -> &mut PaneState {
        match self.active_pane {
            PaneSide::Left => &mut self.left_pane,
            PaneSide::Right => &mut self.right_pane,
        }
    }

    /// アクティブペインの不変参照
    pub fn active_pane_ref(&self) -> &PaneState {
        match self.active_pane {
            PaneSide::Left => &self.left_pane,
            PaneSide::Right => &self.right_pane,
        }
    }

    /// 対向ペインの不変参照
    pub fn opposite_pane_ref(&self) -> &PaneState {
        match self.active_pane {
            PaneSide::Left => &self.right_pane,
            PaneSide::Right => &self.left_pane,
        }
    }

    /// 対向ペインの可変参照
    pub fn opposite_pane_mut(&mut self) -> &mut PaneState {
        match self.active_pane {
            PaneSide::Left => &mut self.right_pane,
            PaneSide::Right => &mut self.left_pane,
        }
    }

    /// 期限切れメッセージを削除
    pub fn cleanup_expired_messages(&mut self) {
        if let Some(ref msg) = self.message {
            if msg.is_expired() {
                self.message = None;
            }
        }
    }

    /// メッセージを表示
    fn show_message(&mut self, text: String, level: MessageLevel) {
        self.message = Some(TimedMessage::new(text, level));
    }

    /// 外部変更の自動検出＆リフレッシュ（2秒間隔）
    ///
    /// 別のターミナルや Finder で行われたファイル操作やブランチ変更を
    /// ディレクトリ・.git/HEAD の mtime 比較で検出し、自動反映する。
    pub fn check_external_changes(&mut self) {
        const AUTO_REFRESH_INTERVAL_SECS: u64 = 2;

        if self.last_auto_refresh.elapsed().as_secs() < AUTO_REFRESH_INTERVAL_SECS {
            return;
        }
        self.last_auto_refresh = Instant::now();

        if matches!(self.mode, AppMode::Normal) {
            if self.left_pane.has_external_changes() {
                let _ = self.left_pane.refresh();
            }
            if self.right_pane.has_external_changes() {
                let _ = self.right_pane.refresh();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Action 適用
    // -----------------------------------------------------------------------

    /// Action を適用してアプリケーション状態を更新する
    ///
    /// テキスト入力モード（コマンド/ダイアログ）から通常モードに戻る際に
    /// IME を ASCII モードに戻す制御を行う。
    pub fn apply_action(&mut self, action: Action) -> Result<(), AppError> {
        let was_text_input = matches!(self.mode, AppMode::Command | AppMode::Search | AppMode::Dialog);
        let result = self.apply_action_inner(action);
        let is_text_input = matches!(self.mode, AppMode::Command | AppMode::Search | AppMode::Dialog);
        // テキスト入力モードから抜けたら ASCII モードを強制
        if was_text_input && !is_text_input {
            crate::ime::force_ascii();
        }
        result
    }

    /// Action の実処理（IME 制御ラッパーから呼ばれる）
    fn apply_action_inner(&mut self, action: Action) -> Result<(), AppError> {
        match action {
            Action::Noop => {}

            // --- ナビゲーション ---
            Action::MoveCursor(delta) => {
                self.active_pane_mut().move_cursor(delta);
            }
            Action::Enter => self.handle_enter()?,
            Action::ParentDirectory => {
                self.active_pane_mut().navigate_parent()?;
                self.reapply_find();
            }
            Action::GoHome => {
                let home = dirs::home_dir().ok_or(AppError::HomeDirNotFound)?;
                self.active_pane_mut().navigate_to(home)?;
                self.reapply_find();
            }
            Action::GoHomeLeft => {
                let home = dirs::home_dir().ok_or(AppError::HomeDirNotFound)?;
                self.left_pane.navigate_to(home)?;
                if self.active_pane == PaneSide::Left {
                    self.reapply_find();
                }
            }
            Action::GoHomeRight => {
                let home = dirs::home_dir().ok_or(AppError::HomeDirNotFound)?;
                self.right_pane.navigate_to(home)?;
                if self.active_pane == PaneSide::Right {
                    self.reapply_find();
                }
            }
            Action::JumpTop => self.active_pane_mut().jump_top(),
            Action::JumpBottom => self.active_pane_mut().jump_bottom(),

            // --- ペイン ---
            Action::SwitchPane => {
                self.active_pane = match self.active_pane {
                    PaneSide::Left => PaneSide::Right,
                    PaneSide::Right => PaneSide::Left,
                };
                self.reapply_find();
            }
            Action::ActivatePane(side) => {
                self.active_pane = side;
                self.reapply_find();
            }
            Action::SetPaneRatio(ratio) => {
                self.pane_ratio = ratio.clamp(0.2, 0.8);
            }
            Action::SyncDirToOtherPane => {
                let dir = self.active_pane_ref().current_dir.clone();
                self.opposite_pane_mut().navigate_to(dir)?;
            }

            // --- ファイル操作 ---
            Action::CopyFiles => self.handle_copy()?,
            Action::MoveFiles => self.handle_move()?,
            Action::RequestDelete => self.handle_request_delete(),
            Action::ExecuteDelete => self.handle_execute_delete()?,
            Action::RequestRename => self.handle_request_rename(),
            Action::ExecuteRename(name) => self.handle_execute_rename(&name)?,
            Action::RequestCreateDir => self.handle_request_create("新規フォルダ", PendingInputAction::CreateDir),
            Action::ExecuteCreateDir(name) => {
                let dir = self.active_pane_ref().current_dir.clone();
                operations::create_dir(&dir, &name)?;
                self.active_pane_mut().refresh()?;
                self.show_message(format!("作成: {}", name), MessageLevel::Success);
            }
            Action::RequestCreateFile => self.handle_request_create("新規ファイル", PendingInputAction::CreateFile),
            Action::RequestChmod => self.handle_request_chmod(),
            Action::ExecuteCreateFile(name) => {
                let dir = self.active_pane_ref().current_dir.clone();
                operations::create_file(&dir, &name)?;
                self.active_pane_mut().refresh()?;
                self.show_message(format!("作成: {}", name), MessageLevel::Success);
            }

            // --- マーキング ---
            Action::ToggleMark => {
                self.active_pane_mut().toggle_mark();
                self.active_pane_mut().move_cursor(1); // マーク後カーソルを1つ下に
            }
            Action::ToggleMarkAll => {
                self.active_pane_mut().toggle_mark_all();
            }

            // --- 表示 ---
            Action::ToggleHidden => {
                let pane = self.active_pane_mut();
                pane.show_hidden = !pane.show_hidden;
                pane.refresh()?;
                self.reapply_find();
            }
            Action::ShowSortMenu => {
                // TODO: ソートメニューダイアログ
            }
            Action::SetSort(criteria) => {
                let pane = self.active_pane_mut();
                pane.sort_order.criteria = criteria;
                pane.refresh()?;
                self.reapply_find();
            }
            Action::ShowFileInfo => {
                // TODO: ファイル情報ポップアップ
            }
            Action::Refresh => {
                self.left_pane.refresh()?;
                self.right_pane.refresh()?;
                self.reapply_find();
            }

            // --- モード遷移 ---
            Action::EnterCommandMode => {
                if self.mode.can_enter_command() {
                    self.command_state.reset();
                    self.mode = AppMode::Command;
                }
            }
            Action::ExitCommandMode => {
                self.mode = AppMode::Normal;
            }
            Action::EnterHelp(topic) => {
                self.help_state = HelpState {
                    scroll_offset: 0,
                    topic,
                };
                self.mode = AppMode::Help;
            }
            Action::ExitHelp => {
                self.mode = AppMode::Normal;
            }

            // --- コマンドモード操作 ---
            Action::CommandInput(c) => self.command_state.insert_char(c),
            Action::CommandBackspace => {
                if self.command_state.input.is_empty() {
                    // 空で Backspace → コマンドモード終了
                    self.mode = AppMode::Normal;
                } else {
                    self.command_state.backspace();
                }
            }
            Action::CommandExecute => self.handle_command_execute()?,
            Action::CommandComplete => self.handle_command_complete(),
            Action::CommandHistoryPrev => self.command_state.history_prev(),
            Action::CommandHistoryNext => self.command_state.history_next(),

            // --- ヘルプ ---
            Action::HelpScroll(delta) => {
                let offset = self.help_state.scroll_offset as i32 + delta;
                self.help_state.scroll_offset = offset.max(0) as usize;
            }

            // --- Git ---
            Action::GitPull => self.handle_git_pull(),
            Action::GitCheckout => self.handle_git_checkout_request(),
            Action::GitBranchList => self.handle_git_branch_list(),
            Action::BranchListMoveCursor(delta) => {
                if let Some(DialogState::BranchList { branches, filter_string, cursor, scroll_offset, .. }) = &mut self.dialog {
                    let filtered = Self::filter_branch_list(branches, filter_string);
                    let len = filtered.len();
                    let max = if len == 0 { 0 } else { len - 1 };
                    let new_pos = (*cursor as i32 + delta).clamp(0, max as i32) as usize;
                    *cursor = new_pos;
                    let visible = 15usize;
                    if *cursor < *scroll_offset {
                        *scroll_offset = *cursor;
                    }
                    if *cursor >= *scroll_offset + visible {
                        *scroll_offset = cursor.saturating_sub(visible - 1);
                    }
                }
            }
            Action::BranchListStartSearch => {
                if let Some(DialogState::BranchList { search_input, .. }) = &mut self.dialog {
                    *search_input = Some(String::new());
                }
            }
            Action::BranchListSearchInput(c) => {
                if let Some(DialogState::BranchList { search_input, .. }) = &mut self.dialog {
                    if let Some(s) = search_input {
                        s.push(c);
                    }
                }
            }
            Action::BranchListSearchBackspace => {
                if let Some(DialogState::BranchList { search_input, .. }) = &mut self.dialog {
                    if let Some(s) = search_input {
                        if !s.is_empty() {
                            let idx = s.char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                            s.truncate(idx);
                        }
                    }
                }
            }
            Action::BranchListSearchEnter => {
                if let Some(DialogState::BranchList { branches, filter_string, search_input, cursor, scroll_offset }) = &mut self.dialog {
                    let new_filter = search_input.take().unwrap_or_default();
                    *filter_string = new_filter.clone();
                    let filtered = Self::filter_branch_list(branches, &new_filter);
                    *cursor = 0;
                    *scroll_offset = 0;
                    if !filtered.is_empty() && *cursor >= filtered.len() {
                        *cursor = filtered.len() - 1;
                    }
                }
            }
            Action::BranchListSearchCancel => {
                if let Some(DialogState::BranchList { search_input, .. }) = &mut self.dialog {
                    *search_input = None;
                }
            }

            // --- 検索 ---
            Action::FindNext => {
                let info = self.find_state.as_mut().and_then(|fs| {
                    if fs.matches.is_empty() {
                        return None;
                    }
                    fs.current = (fs.current + 1) % fs.matches.len();
                    Some((fs.matches[fs.current], fs.current + 1, fs.matches.len(), fs.query.clone()))
                });
                if let Some((idx, pos, total, query)) = info {
                    self.active_pane_mut().cursor = idx;
                    self.show_message(
                        format!("「{}」検索結果 ({}/{})", query, pos, total),
                        MessageLevel::Info,
                    );
                }
            }
            Action::FindPrev => {
                let info = self.find_state.as_mut().and_then(|fs| {
                    if fs.matches.is_empty() {
                        return None;
                    }
                    fs.current = if fs.current == 0 {
                        fs.matches.len() - 1
                    } else {
                        fs.current - 1
                    };
                    Some((fs.matches[fs.current], fs.current + 1, fs.matches.len(), fs.query.clone()))
                });
                if let Some((idx, pos, total, query)) = info {
                    self.active_pane_mut().cursor = idx;
                    self.show_message(
                        format!("「{}」検索結果 ({}/{})", query, pos, total),
                        MessageLevel::Info,
                    );
                }
            }

            // --- 検索モード ---
            Action::EnterSearchMode => {
                if self.mode == AppMode::Normal {
                    self.search_input.clear();
                    self.mode = AppMode::Search;
                }
            }
            Action::SearchInput(c) => {
                self.search_input.push(c);
            }
            Action::SearchBackspace => {
                if self.search_input.is_empty() {
                    self.mode = AppMode::Normal;
                } else {
                    let idx = self.search_input.char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                    self.search_input.truncate(idx);
                }
            }
            Action::SearchExecute => {
                let query = self.search_input.clone();
                self.mode = AppMode::Normal;
                if query.is_empty() {
                    self.search_query = None;
                    self.find_state = None;
                    self.show_message("検索をクリアしました".to_string(), MessageLevel::Info);
                } else {
                    self.search_query = Some(query.clone());
                    self.execute_find(&query);
                }
            }
            Action::ExitSearchMode => {
                self.mode = AppMode::Normal;
            }
            Action::ClearSearch => {
                self.search_query = None;
                self.find_state = None;
                self.show_message("検索をクリアしました".to_string(), MessageLevel::Info);
            }

            // --- クリップボード ---
            Action::CopyFileName => {
                if let Some(entry) = self.active_pane_ref().current_entry() {
                    if entry.name != "." && entry.name != ".." {
                        let name = entry.name.clone();
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&name)) {
                            Ok(_) => self.show_message(format!("コピー: {}", name), MessageLevel::Success),
                            Err(e) => self.show_message(format!("クリップボードエラー: {}", e), MessageLevel::Error),
                        }
                    }
                }
            }
            Action::CopyFullPath => {
                if let Some(entry) = self.active_pane_ref().current_entry() {
                    if entry.name != "." && entry.name != ".." {
                        let path = entry.path.to_string_lossy().to_string();
                        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&path)) {
                            Ok(_) => self.show_message(format!("コピー: {}", path), MessageLevel::Success),
                            Err(e) => self.show_message(format!("クリップボードエラー: {}", e), MessageLevel::Error),
                        }
                    }
                }
            }

            // --- ダイアログ ---
            Action::ShowConfirmDialog { title, message } => {
                // ダイアログ表示用（内部では handle_request_delete 等で使用）
                self.mode = AppMode::Dialog;
                self.dialog = Some(DialogState::Confirm {
                    title,
                    message,
                    focus_yes: false, // デフォルト No（安全側）
                    pending: PendingAction::Delete(vec![]),
                });
            }
            Action::DialogConfirm => self.handle_dialog_confirm()?,
            Action::DialogCancel => {
                self.dialog = None;
                self.mode = AppMode::Normal;
            }
            Action::DialogConflictNext => {
                if let Some(DialogState::CopyMoveConflict { focus, .. }) = &mut self.dialog {
                    *focus = (*focus + 1) % 3;
                }
            }
            Action::DialogConflictPrev => {
                if let Some(DialogState::CopyMoveConflict { focus, .. }) = &mut self.dialog {
                    *focus = (*focus + 2) % 3;
                }
            }
            Action::DialogInput(c) => {
                if let Some(DialogState::Input { value, cursor_pos, .. }) = &mut self.dialog {
                    value.insert(*cursor_pos, c);
                    *cursor_pos += c.len_utf8();
                }
            }
            Action::DialogBackspace => {
                if let Some(DialogState::Input { value, cursor_pos, .. }) = &mut self.dialog {
                    if *cursor_pos > 0 {
                        let prev = value[..*cursor_pos]
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        value.remove(prev);
                        *cursor_pos = prev;
                    }
                }
            }
            Action::DialogCursorLeft => {
                if let Some(DialogState::Input { value, cursor_pos, .. }) = &mut self.dialog {
                    if *cursor_pos > 0 {
                        // 前の文字境界へ移動
                        *cursor_pos = value[..*cursor_pos]
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                    }
                }
            }
            Action::DialogCursorRight => {
                if let Some(DialogState::Input { value, cursor_pos, .. }) = &mut self.dialog {
                    if *cursor_pos < value.len() {
                        // 次の文字境界へ移動
                        *cursor_pos = value[*cursor_pos..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| *cursor_pos + i)
                            .unwrap_or(value.len());
                    }
                }
            }
            Action::DialogHome => {
                if let Some(DialogState::Input { cursor_pos, .. }) = &mut self.dialog {
                    *cursor_pos = 0;
                }
            }
            Action::DialogEnd => {
                if let Some(DialogState::Input { value, cursor_pos, .. }) = &mut self.dialog {
                    *cursor_pos = value.len();
                }
            }

            // --- アプリ制御 ---
            Action::Quit => {
                let _ = self.command_state.history.save();
                self.should_quit = true;
            }
            Action::ShowMessage(text, level) => {
                self.show_message(text, level);
            }
            Action::DismissWelcome => {
                self.show_welcome = false;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // アクションハンドラ（内部）
    // -----------------------------------------------------------------------

    /// Enter: ディレクトリなら入る、ファイルならデフォルトアプリで開く
    ///
    /// `.` はリフレッシュ、`..` は親ディレクトリへ移動として処理する。
    fn handle_enter(&mut self) -> Result<(), AppError> {
        let entry = match self.active_pane_ref().current_entry() {
            Some(e) => e.clone(),
            None => return Ok(()),
        };

        // . エントリ: カレントディレクトリをリフレッシュ
        if entry.name == "." {
            self.active_pane_mut().refresh()?;
            self.reapply_find();
            return Ok(());
        }

        // .. エントリ: 親ディレクトリへ移動
        if entry.name == ".." {
            self.active_pane_mut().navigate_parent()?;
            self.reapply_find();
            return Ok(());
        }

        if entry.is_dir() {
            let path = entry.path.clone();
            self.active_pane_mut().navigate_to(path)?;
            self.reapply_find();
        } else {
            // デフォルトアプリでファイルを開く
            if let Err(e) = open::that(&entry.path) {
                self.show_message(format!("開けません: {}", e), MessageLevel::Error);
            }
        }
        Ok(())
    }

    /// コピー実行（同名ファイルがある場合は衝突ダイアログを表示）
    fn handle_copy(&mut self) -> Result<(), AppError> {
        let sources = self.active_pane_ref().selected_paths();
        if sources.is_empty() {
            return Ok(());
        }
        let dest_dir = self.opposite_pane_ref().current_dir.clone();
        if let Some(index) = sources.iter().position(|s| {
            s.file_name()
                .map(|n| dest_dir.join(n).exists())
                .unwrap_or(false)
        }) {
            self.mode = AppMode::Dialog;
            self.dialog = Some(DialogState::CopyMoveConflict {
                sources,
                dest_dir,
                index,
                is_move: false,
                focus: 0,
            });
            return Ok(());
        }
        let count = operations::copy_files(&sources, &dest_dir, &|_| ConflictResolution::Overwrite)?;
        self.active_pane_mut().clear_marks();
        self.opposite_pane_mut().refresh()?;
        self.show_message(format!("{}件コピーしました", count), MessageLevel::Success);
        Ok(())
    }

    /// 移動実行（同名ファイルがある場合は衝突ダイアログを表示）
    fn handle_move(&mut self) -> Result<(), AppError> {
        let sources = self.active_pane_ref().selected_paths();
        if sources.is_empty() {
            return Ok(());
        }
        let dest_dir = self.opposite_pane_ref().current_dir.clone();
        if let Some(index) = sources.iter().position(|s| {
            s.file_name()
                .map(|n| dest_dir.join(n).exists())
                .unwrap_or(false)
        }) {
            self.mode = AppMode::Dialog;
            self.dialog = Some(DialogState::CopyMoveConflict {
                sources,
                dest_dir,
                index,
                is_move: true,
                focus: 0,
            });
            return Ok(());
        }
        let count = operations::move_files(&sources, &dest_dir, &|_| ConflictResolution::Overwrite)?;
        self.active_pane_mut().clear_marks();
        self.active_pane_mut().refresh()?;
        self.opposite_pane_mut().refresh()?;
        self.show_message(format!("{}件移動しました", count), MessageLevel::Success);
        Ok(())
    }

    /// 同名を避けるためのユニークなパスを生成（例: file.txt → file (1).txt）
    fn unique_dest_path(dest_dir: &std::path::Path, source: &std::path::Path) -> PathBuf {
        let name = match source.file_name() {
            Some(n) => n.to_string_lossy(),
            None => return dest_dir.join("unknown"),
        };
        let stem = source
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| name.clone().into_owned());
        let ext = source.extension().map(|e| format!(".{}", e.to_string_lossy()));

        let first = dest_dir.join(&*name);
        if !first.exists() {
            return first;
        }
        for i in 1..1000 {
            let new_name = match &ext {
                Some(e) => format!("{} ({}){}", stem, i, e),
                None => format!("{} ({})", stem, i),
            };
            let p = dest_dir.join(&new_name);
            if !p.exists() {
                return p;
            }
        }
        dest_dir.join(name.to_string() + "_copy")
    }

    /// 削除確認ダイアログ表示
    fn handle_request_delete(&mut self) {
        let paths = self.active_pane_ref().selected_paths();
        if paths.is_empty() {
            return;
        }
        let msg = if paths.len() == 1 {
            format!(
                "「{}」を削除しますか？（ゴミ箱へ移動）",
                paths[0].file_name().unwrap_or_default().to_string_lossy()
            )
        } else {
            format!("{}件を削除しますか？（ゴミ箱へ移動）", paths.len())
        };

        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Confirm {
            title: "削除".to_string(),
            message: msg,
            focus_yes: false,
            pending: PendingAction::Delete(paths),
        });
    }

    /// 削除実行
    fn handle_execute_delete(&mut self) -> Result<(), AppError> {
        let paths = self.active_pane_ref().selected_paths();
        let count = operations::delete_to_trash(&paths)?;
        self.active_pane_mut().clear_marks();
        self.active_pane_mut().refresh()?;
        self.show_message(format!("{}件削除しました", count), MessageLevel::Success);
        Ok(())
    }

    /// リネームダイアログ表示
    ///
    /// `.` と `..` はリネーム不可。
    fn handle_request_rename(&mut self) {
        let entry = match self.active_pane_ref().current_entry() {
            Some(e) => e.clone(),
            None => return,
        };
        // . と .. はリネーム不可
        if entry.name == "." || entry.name == ".." {
            return;
        }
        let name = entry.name.clone();
        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Input {
            title: "名前変更".to_string(),
            value: name.clone(),
            cursor_pos: name.len(),
            pending: PendingInputAction::Rename(entry.path.clone()),
        });
    }

    /// リネーム実行
    fn handle_execute_rename(&mut self, new_name: &str) -> Result<(), AppError> {
        if let Some(entry) = self.active_pane_ref().current_entry() {
            let path = entry.path.clone();
            operations::rename(&path, new_name)?;
            self.active_pane_mut().refresh()?;
            self.show_message(format!("名前変更: {}", new_name), MessageLevel::Success);
        }
        Ok(())
    }

    /// パーミッション変更ダイアログ表示（Unix のみ）
    ///
    /// 現在のパーミッションを8進数で表示し、ユーザーに新しい値を入力させる。
    /// Windows では非対応メッセージを表示する。
    fn handle_request_chmod(&mut self) {
        if cfg!(not(unix)) {
            self.show_message(
                "chmod は Unix 系 OS でのみ使用可能です".to_string(),
                MessageLevel::Warning,
            );
            return;
        }

        let entry = match self.active_pane_ref().current_entry() {
            Some(e) => e.clone(),
            None => return,
        };
        // . と .. は対象外
        if entry.name == "." || entry.name == ".." {
            return;
        }
        let paths = self.active_pane_ref().selected_paths();
        if paths.is_empty() {
            return;
        }

        // 現在のパーミッションを8進数で表示（例: "755"）
        let current_mode = format!("{:o}", entry.mode & 0o7777);
        let cursor_pos = current_mode.len();

        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Input {
            title: "パーミッション変更（8進数）".to_string(),
            value: current_mode,
            cursor_pos,
            pending: PendingInputAction::Chmod(paths),
        });
    }

    /// 作成ダイアログ表示
    fn handle_request_create(&mut self, title: &str, pending: PendingInputAction) {
        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Input {
            title: title.to_string(),
            value: String::new(),
            cursor_pos: 0,
            pending,
        });
    }

    /// ダイアログ確認処理
    fn handle_dialog_confirm(&mut self) -> Result<(), AppError> {
        let dialog = self.dialog.take();
        self.mode = AppMode::Normal;

        match dialog {
            Some(DialogState::CopyMoveConflict {
                sources,
                dest_dir,
                index,
                is_move,
                focus,
            }) => {
                if focus == 2 {
                    self.active_pane_mut().clear_marks();
                    let _ = self.active_pane_mut().refresh();
                    let _ = self.opposite_pane_mut().refresh();
                    self.show_message("キャンセルしました".to_string(), MessageLevel::Info);
                    return Ok(());
                }
                let op = if is_move {
                    operations::move_files
                } else {
                    operations::copy_files
                };
                if focus == 0 {
                    op(&[sources[index].clone()], &dest_dir, &|_| ConflictResolution::Overwrite)?;
                } else {
                    let new_path = Self::unique_dest_path(&dest_dir, &sources[index]);
                    op(
                        &[sources[index].clone()],
                        &dest_dir,
                        &|_| ConflictResolution::Rename(new_path.clone()),
                    )?;
                }
                let mut idx = index + 1;
                while idx < sources.len() {
                    let dest_path = dest_dir.join(sources[idx].file_name().unwrap_or_default());
                    if dest_path.exists() {
                        self.mode = AppMode::Dialog;
                        self.dialog = Some(DialogState::CopyMoveConflict {
                            sources,
                            dest_dir,
                            index: idx,
                            is_move,
                            focus: 0,
                        });
                        return Ok(());
                    }
                    op(&[sources[idx].clone()], &dest_dir, &|_| ConflictResolution::Overwrite)?;
                    idx += 1;
                }
                self.active_pane_mut().clear_marks();
                let _ = self.active_pane_mut().refresh();
                let _ = self.opposite_pane_mut().refresh();
                let verb = if is_move { "移動" } else { "コピー" };
                self.show_message(
                    format!("{}件{}しました", sources.len(), verb),
                    MessageLevel::Success,
                );
            }
            Some(DialogState::Confirm { pending, .. }) => match pending {
                PendingAction::Delete(paths) => {
                    let count = operations::delete_to_trash(&paths)?;
                    self.active_pane_mut().clear_marks();
                    self.active_pane_mut().refresh()?;
                    self.show_message(
                        format!("{}件削除しました", count),
                        MessageLevel::Success,
                    );
                }
            },
            Some(DialogState::Message { .. }) => {
                // Message ダイアログは確定操作なし（ESC で閉じるのみ）
            }
            Some(DialogState::BranchList { branches, filter_string, cursor, .. }) => {
                let filtered = Self::filter_branch_list(&branches, &filter_string);
                if let Some((display, is_current, checkout_arg)) = filtered.get(cursor) {
                    if *is_current {
                        self.show_message(
                            format!("既に {} ブランチにいます", display),
                            MessageLevel::Info,
                        );
                    } else {
                        self.handle_git_checkout_execute(checkout_arg);
                    }
                }
            }
            Some(DialogState::Input { value, pending, .. }) => {
                if value.trim().is_empty() {
                    return Ok(());
                }
                match pending {
                    PendingInputAction::Rename(path) => {
                        operations::rename(&path, &value)?;
                        self.active_pane_mut().refresh()?;
                        self.show_message(
                            format!("名前変更: {}", value),
                            MessageLevel::Success,
                        );
                    }
                    PendingInputAction::CreateDir => {
                        let dir = self.active_pane_ref().current_dir.clone();
                        operations::create_dir(&dir, &value)?;
                        self.active_pane_mut().refresh()?;
                        self.show_message(
                            format!("作成: {}", value),
                            MessageLevel::Success,
                        );
                    }
                    PendingInputAction::CreateFile => {
                        let dir = self.active_pane_ref().current_dir.clone();
                        operations::create_file(&dir, &value)?;
                        self.active_pane_mut().refresh()?;
                        self.show_message(
                            format!("作成: {}", value),
                            MessageLevel::Success,
                        );
                    }
                    PendingInputAction::GitCheckout => {
                        self.handle_git_checkout_execute(&value);
                    }
                    PendingInputAction::Chmod(paths) => {
                        match u32::from_str_radix(value.trim(), 8) {
                            Ok(mode) if mode <= 0o7777 => {
                                let count = operations::chmod(&paths, mode)?;
                                self.active_pane_mut().refresh()?;
                                self.show_message(
                                    format!("{}件のパーミッションを変更しました", count),
                                    MessageLevel::Success,
                                );
                            }
                            _ => {
                                self.show_message(
                                    format!(
                                        "無効なパーミッション: 「{}」（例: 755, 644）",
                                        value
                                    ),
                                    MessageLevel::Error,
                                );
                            }
                        }
                    }
                }
            }
            None => {}
        }

        Ok(())
    }

    /// git pull 実行
    fn handle_git_pull(&mut self) {
        let dir = self.active_pane_ref().current_dir.clone();
        if !crate::utils::git::is_git_repo(&dir) {
            self.show_message(
                "Git リポジトリではありません".to_string(),
                MessageLevel::Warning,
            );
            return;
        }

        let result = crate::utils::git::git_pull(&dir);
        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Message {
            title: "git pull".to_string(),
            content: result,
        });

        // pull 後にファイル一覧を更新
        let _ = self.left_pane.refresh();
        let _ = self.right_pane.refresh();
    }

    /// キーワードでブランチリストをフィルタ（大文字小文字無視）
    fn filter_branch_list(
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

    /// git branch -a リストダイアログ表示
    fn handle_git_branch_list(&mut self) {
        let dir = self.active_pane_ref().current_dir.clone();
        if !crate::utils::git::is_git_repo(&dir) {
            self.show_message(
                "Git リポジトリではありません".to_string(),
                MessageLevel::Warning,
            );
            return;
        }

        let items = crate::utils::git::git_branch_list(&dir);
        if items.is_empty() {
            self.show_message(
                "ブランチが見つかりません".to_string(),
                MessageLevel::Warning,
            );
            return;
        }

        let branch_tuples: Vec<(String, bool, String)> = items
            .into_iter()
            .map(|b| (b.display, b.is_current, b.checkout_arg))
            .collect();
        let current_idx = branch_tuples.iter().position(|(_, is_cur, _)| *is_cur).unwrap_or(0);

        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::BranchList {
            branches: branch_tuples,
            filter_string: String::new(),
            search_input: None,
            cursor: current_idx,
            scroll_offset: 0,
        });
    }

    /// git checkout 入力ダイアログ表示
    fn handle_git_checkout_request(&mut self) {
        let dir = self.active_pane_ref().current_dir.clone();
        if !crate::utils::git::is_git_repo(&dir) {
            self.show_message(
                "Git リポジトリではありません".to_string(),
                MessageLevel::Warning,
            );
            return;
        }

        self.mode = AppMode::Dialog;
        self.dialog = Some(DialogState::Input {
            title: "git checkout（ブランチ名または -b 新ブランチ名）".to_string(),
            value: String::new(),
            cursor_pos: 0,
            pending: PendingInputAction::GitCheckout,
        });
    }

    /// git checkout 実行
    fn handle_git_checkout_execute(&mut self, args: &str) {
        let dir = self.active_pane_ref().current_dir.clone();
        let result = crate::utils::git::git_checkout(&dir, args);

        if result.success {
            // refresh() 内で git_branch も更新される
            let _ = self.left_pane.refresh();
            let _ = self.right_pane.refresh();
            self.show_message(
                format!("git checkout: {}", result.message),
                MessageLevel::Success,
            );
        } else {
            self.mode = AppMode::Dialog;
            self.dialog = Some(DialogState::Message {
                title: "git checkout エラー".to_string(),
                content: result.message,
            });
        }
    }

    /// コマンド実行
    fn handle_command_execute(&mut self) -> Result<(), AppError> {
        let input = self.command_state.input.clone();
        self.command_state.history.push(&input);
        self.mode = AppMode::Normal;

        let parse_result = command::parser::parse(&input);
        let current_dir = self.active_pane_ref().current_dir.clone();

        match command::executor::execute(&parse_result, &current_dir) {
            Ok(cmd_action) => {
                use command::executor::CommandAction;
                match cmd_action {
                    CommandAction::ShowHelp(topic) => {
                        self.help_state = HelpState {
                            scroll_offset: 0,
                            topic,
                        };
                        self.mode = AppMode::Help;
                    }
                    CommandAction::Quit => {
                        let _ = self.command_state.history.save();
                        self.should_quit = true;
                    }
                    CommandAction::ChangeDir(path) => {
                        self.active_pane_mut().navigate_to(path)?;
                    }
                    CommandAction::SetSort(criteria) => {
                        let pane = self.active_pane_mut();
                        pane.sort_order.criteria = criteria;
                        pane.refresh()?;
                    }
                    CommandAction::ToggleHidden => {
                        let pane = self.active_pane_mut();
                        pane.show_hidden = !pane.show_hidden;
                        pane.refresh()?;
                    }
                    // CommandAction::Shell(cmd) => {
                    //     self.show_message(
                    //         format!("シェル: {}", cmd),
                    //         MessageLevel::Info,
                    //     );
                    // }
                    CommandAction::CreateDir(name) => {
                        let dir = self.active_pane_ref().current_dir.clone();
                        operations::create_dir(&dir, &name)?;
                        self.active_pane_mut().refresh()?;
                        self.show_message(format!("作成: {}", name), MessageLevel::Success);
                    }
                    CommandAction::CreateFile(name) => {
                        let dir = self.active_pane_ref().current_dir.clone();
                        operations::create_file(&dir, &name)?;
                        self.active_pane_mut().refresh()?;
                        self.show_message(format!("作成: {}", name), MessageLevel::Success);
                    }
                    CommandAction::Find(query) => {
                        self.execute_find(&query);
                    }
                }
            }
            Err(e) => {
                self.show_message(e.user_message(), MessageLevel::Error);
            }
        }

        Ok(())
    }

    /// ファイル名検索を実行
    ///
    /// アクティブペインのエントリから、名前に検索文字列を含むものを検索。
    /// 見つかった場合はカーソルを最初のマッチに移動し、find_state を設定する。
    /// search_query も更新し、ペイン切替・ディレクトリ移動後に自動再検索する。
    fn execute_find(&mut self, query: &str) {
        self.search_query = Some(query.to_string());

        let query_lower = query.to_lowercase();
        let matches: Vec<usize> = self
            .active_pane_ref()
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.name != "." && e.name != ".." && e.name.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();

        if matches.is_empty() {
            self.find_state = None;
            self.show_message(
                format!("「{}」は見つかりませんでした", query),
                MessageLevel::Warning,
            );
        } else {
            let first = matches[0];
            let total = matches.len();
            self.find_state = Some(FindState {
                query: query.to_string(),
                matches,
                current: 0,
            });
            self.active_pane_mut().cursor = first;
            self.show_message(
                format!("「{}」{}件見つかりました (1/{})", query, total, total),
                MessageLevel::Info,
            );
        }
    }

    /// 永続検索クエリを再適用（ペイン切替・ディレクトリ移動後に呼ぶ）
    ///
    /// カーソル位置は変更せず、find_state のみ更新する。
    fn reapply_find(&mut self) {
        let query = match &self.search_query {
            Some(q) => q.clone(),
            None => {
                self.find_state = None;
                return;
            }
        };

        let query_lower = query.to_lowercase();
        let matches: Vec<usize> = self
            .active_pane_ref()
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.name != "." && e.name != ".." && e.name.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();

        if matches.is_empty() {
            self.find_state = None;
        } else {
            self.find_state = Some(FindState {
                query,
                matches,
                current: 0,
            });
        }
    }

    /// オートコンプリート
    fn handle_command_complete(&mut self) {
        let current_dir = self.active_pane_ref().current_dir.clone();
        let input = &self.command_state.input;

        if self.command_state.completions.is_empty() {
            // 候補生成
            self.command_state.completions =
                command::completer::complete(input, &current_dir);
            self.command_state.completion_index = None;
        }

        if self.command_state.completions.is_empty() {
            return;
        }

        // 次の候補を選択
        let idx = match self.command_state.completion_index {
            None => 0,
            Some(i) => (i + 1) % self.command_state.completions.len(),
        };
        self.command_state.completion_index = Some(idx);

        // 入力を補完で置き換え
        let completion = self.command_state.completions[idx].clone();
        let parts: Vec<&str> = self.command_state.input.splitn(2, ' ').collect();
        if parts.len() == 2 {
            self.command_state.input = format!("{} {}", parts[0], completion);
        } else {
            self.command_state.input = completion;
        }
        self.command_state.cursor_pos = self.command_state.input.len();
    }
}
