# mdir - 詳細設計書

**バージョン:** 0.1.0
**作成日:** 2026-02-11
**対応要件:** REQUIREMENTS.md
**対応基本設計:** docs/BASIC_DESIGN.md
**対応UI設計:** docs/UI_DESIGN.md
**ステータス:** ドラフト

---

## 1. 型定義

本章ではアプリケーション全体で使用する主要な型をRustコードとして定義する。

### 1.1 アプリケーション状態 (`app.rs`)

```rust
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

/// アプリケーション全体の状態を保持するルート構造体
pub struct App {
    /// 現在のアプリケーションモード
    pub mode: AppMode,
    /// 左ペインの状態
    pub left_pane: PaneState,
    /// 右ペインの状態
    pub right_pane: PaneState,
    /// アクティブペイン
    pub active_pane: PaneSide,
    /// コマンドモードの状態
    pub command_state: CommandState,
    /// ダイアログの状態（表示中のみSome）
    pub dialog: Option<DialogState>,
    /// ヘルプ画面の状態
    pub help_state: HelpState,
    /// ペイン幅比率（0.2 - 0.8）
    pub pane_ratio: f32,
    /// ユーザー設定
    pub config: Config,
    /// 一時メッセージ（ステータスバー/コマンドラインに表示）
    pub message: Option<TimedMessage>,
    /// 終了フラグ
    pub should_quit: bool,
    /// 初回起動フラグ
    pub first_launch: bool,
    /// ペイン境界ドラッグ中フラグ
    pub dragging_border: bool,
    /// 直前のクリック情報（ダブルクリック判定用）
    pub last_click: Option<ClickInfo>,
}

/// ペインの左右
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSide {
    Left,
    Right,
}

/// 一時メッセージ（自動消去付き）
pub struct TimedMessage {
    pub text: String,
    pub level: MessageLevel,
    pub created_at: Instant,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// ダブルクリック判定用のクリック情報
pub struct ClickInfo {
    pub x: u16,
    pub y: u16,
    pub time: Instant,
}
```

### 1.2 モード (`mode.rs`)

```rust
/// アプリケーションモード（viのモード概念に対応）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// 通常モード - ファイルブラウジング・操作
    Normal,
    /// コマンドモード - ':' プロンプトでコマンド入力
    Command,
    /// ヘルプモード - 全画面ヘルプ表示
    Help,
    /// ダイアログモード - モーダルダイアログ表示中
    Dialog,
}

impl AppMode {
    /// コマンドモードへの遷移が可能か
    pub fn can_enter_command(&self) -> bool {
        matches!(self, AppMode::Normal)
    }

    /// ヘルプモードへの遷移が可能か
    pub fn can_enter_help(&self) -> bool {
        matches!(self, AppMode::Normal | AppMode::Command)
    }
}
```

### 1.3 ペイン状態 (`fs/mod.rs`)

```rust
use std::collections::HashSet;
use std::path::PathBuf;

/// 1つのファイルペインの状態
pub struct PaneState {
    /// カレントディレクトリの絶対パス
    pub current_dir: PathBuf,
    /// 表示用エントリ一覧（ソート・フィルタ適用済み）
    pub entries: Vec<FileEntry>,
    /// 全エントリ（フィルタ前の完全一覧）
    pub all_entries: Vec<FileEntry>,
    /// カーソル位置（0-indexed）
    pub cursor: usize,
    /// スクロールオフセット（表示先頭行のインデックス）
    pub scroll_offset: usize,
    /// マーク済みファイルのインデックスセット
    pub marked: HashSet<usize>,
    /// ソート設定
    pub sort_order: SortOrder,
    /// 隠しファイル表示フラグ
    pub show_hidden: bool,
    /// フィルタ文字列（Noneはフィルタなし）
    pub filter: Option<String>,
}

impl PaneState {
    /// 指定ディレクトリで初期化
    pub fn new(path: PathBuf) -> Result<Self, AppError> { /* ... */ }

    /// ディレクトリ内容を再読み込み
    pub fn refresh(&mut self) -> Result<(), AppError> { /* ... */ }

    /// カーソルを安全に移動（範囲外に出ない）
    pub fn move_cursor(&mut self, delta: i32) { /* ... */ }

    /// カーソル位置のエントリを取得
    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// マーク済みエントリ一覧を取得（なければカーソル位置のエントリ）
    pub fn selected_entries(&self) -> Vec<&FileEntry> { /* ... */ }

    /// ソートとフィルタを再適用
    pub fn apply_sort_and_filter(&mut self) { /* ... */ }

    /// スクロール位置をカーソル位置に合わせて調整
    pub fn adjust_scroll(&mut self, visible_height: usize) { /* ... */ }
}
```

### 1.4 ファイルエントリ (`fs/entry.rs`)

```rust
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::SystemTime;

/// ファイル/ディレクトリの種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
    /// ディレクトリ
    Directory,
    /// 通常ファイル
    File,
    /// 実行可能ファイル
    Executable,
    /// シンボリックリンク
    Symlink,
}

/// ファイルエントリ（1ファイル/ディレクトリの情報）
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// ファイル名
    pub name: String,
    /// 絶対パス
    pub path: PathBuf,
    /// エントリ種別
    pub entry_type: EntryType,
    /// ファイルサイズ（バイト）
    pub size: u64,
    /// 最終更新日時
    pub modified: SystemTime,
    /// 作成日時
    pub created: Option<SystemTime>,
    /// Unixパーミッション（モードビット）
    pub mode: u32,
    /// 隠しファイルフラグ（名前が'.'で始まる）
    pub is_hidden: bool,
    /// シンボリックリンクフラグ
    pub is_symlink: bool,
    /// シンボリックリンクの参照先
    pub symlink_target: Option<PathBuf>,
    /// 拡張子（ソート用にキャッシュ）
    pub extension: Option<String>,
}

impl FileEntry {
    /// std::fs::DirEntry から FileEntry を構築
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> Result<Self, std::io::Error> {
        let metadata = entry.metadata()?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().canonicalize().unwrap_or_else(|_| entry.path());
        let is_hidden = name.starts_with('.');

        let entry_type = if metadata.is_dir() {
            EntryType::Directory
        } else if metadata.is_symlink() {
            EntryType::Symlink
        } else if metadata.permissions().mode() & 0o111 != 0 {
            EntryType::Executable
        } else {
            EntryType::File
        };

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());

        let symlink_target = if metadata.is_symlink() {
            std::fs::read_link(&path).ok()
        } else {
            None
        };

        Ok(Self {
            name,
            path,
            entry_type,
            size: metadata.len(),
            modified: metadata.modified()?,
            created: metadata.created().ok(),
            mode: metadata.permissions().mode(),
            is_hidden,
            is_symlink: metadata.is_symlink(),
            symlink_target,
            extension,
        })
    }

    /// パーミッションを "-rwxr-xr-x" 形式の文字列に変換
    pub fn permission_string(&self) -> String {
        let m = self.mode;
        let file_type = match self.entry_type {
            EntryType::Directory => 'd',
            EntryType::Symlink => 'l',
            _ => '-',
        };
        format!(
            "{}{}{}{}{}{}{}{}{}{}",
            file_type,
            if m & 0o400 != 0 { 'r' } else { '-' },
            if m & 0o200 != 0 { 'w' } else { '-' },
            if m & 0o100 != 0 { 'x' } else { '-' },
            if m & 0o040 != 0 { 'r' } else { '-' },
            if m & 0o020 != 0 { 'w' } else { '-' },
            if m & 0o010 != 0 { 'x' } else { '-' },
            if m & 0o004 != 0 { 'r' } else { '-' },
            if m & 0o002 != 0 { 'w' } else { '-' },
            if m & 0o001 != 0 { 'x' } else { '-' },
        )
    }
}
```

### 1.5 ソート (`fs/entry.rs` 内)

```rust
/// ソート基準
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortCriteria {
    Name,
    Size,
    Date,
    Extension,
}

/// ソート設定
#[derive(Debug, Clone)]
pub struct SortOrder {
    pub criteria: SortCriteria,
    pub ascending: bool,
    pub dirs_first: bool,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self {
            criteria: SortCriteria::Name,
            ascending: true,
            dirs_first: true,
        }
    }
}

impl SortOrder {
    /// エントリ一覧をソートする
    pub fn sort(&self, entries: &mut Vec<FileEntry>) {
        entries.sort_by(|a, b| {
            // 1. ディレクトリ優先（有効な場合）
            if self.dirs_first {
                let a_is_dir = matches!(a.entry_type, EntryType::Directory);
                let b_is_dir = matches!(b.entry_type, EntryType::Directory);
                if a_is_dir != b_is_dir {
                    return if a_is_dir {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    };
                }
            }

            // 2. 指定基準でソート
            let ord = match self.criteria {
                SortCriteria::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                SortCriteria::Size => a.size.cmp(&b.size),
                SortCriteria::Date => a.modified.cmp(&b.modified),
                SortCriteria::Extension => {
                    a.extension.as_deref().unwrap_or("")
                        .cmp(b.extension.as_deref().unwrap_or(""))
                }
            };

            // 3. 昇順/降順の反転
            if self.ascending { ord } else { ord.reverse() }
        });
    }
}
```

---

## 2. アクションとイベント処理

### 2.1 アクション型定義 (`app.rs`)

```rust
/// アプリケーション状態を変更するアクション
/// イベントハンドラが生成し、App が適用する
#[derive(Debug)]
pub enum Action {
    // === ナビゲーション ===
    /// カーソル移動
    MoveCursor(i32),
    /// ディレクトリに入る / ファイルを開く
    Enter,
    /// 親ディレクトリに戻る
    ParentDirectory,
    /// ホームディレクトリに移動
    GoHome,
    /// 先頭にジャンプ
    JumpTop,
    /// 末尾にジャンプ
    JumpBottom,
    /// 指定パスに移動
    ChangeDirectory(PathBuf),

    // === ペイン ===
    /// アクティブペインを切り替え
    SwitchPane,
    /// 指定ペインをアクティブに
    ActivatePane(PaneSide),
    /// ペイン幅比率を変更
    SetPaneRatio(f32),

    // === ファイル操作 ===
    /// 選択ファイルをコピー
    CopyFiles,
    /// 選択ファイルを移動
    MoveFiles,
    /// 選択ファイルを削除（確認ダイアログ経由）
    RequestDelete,
    /// 削除を実行（ダイアログ確認後）
    ExecuteDelete,
    /// リネーム開始（入力ダイアログ表示）
    RequestRename,
    /// リネーム実行
    ExecuteRename(String),
    /// ディレクトリ作成開始
    RequestCreateDir,
    /// ディレクトリ作成実行
    ExecuteCreateDir(String),
    /// ファイル作成開始
    RequestCreateFile,
    /// ファイル作成実行
    ExecuteCreateFile(String),

    // === マーキング ===
    /// カーソル位置のマーク切替
    ToggleMark,
    /// 全選択/全解除
    ToggleMarkAll,

    // === 表示 ===
    /// 隠しファイル表示切替
    ToggleHidden,
    /// ソートメニュー表示
    ShowSortMenu,
    /// ソート設定変更
    SetSort(SortOrder),
    /// ファイル情報ポップアップ表示
    ShowFileInfo,
    /// ファイル一覧リフレッシュ
    Refresh,

    // === モード遷移 ===
    /// コマンドモードに入る
    EnterCommandMode,
    /// コマンドモードを終了
    ExitCommandMode,
    /// ヘルプモードに入る
    EnterHelp(Option<String>),
    /// ヘルプモードを終了
    ExitHelp,

    // === コマンドモード操作 ===
    /// コマンド文字入力
    CommandInput(char),
    /// コマンド文字削除（Backspace）
    CommandBackspace,
    /// コマンド実行（Enter）
    CommandExecute,
    /// Tab補完
    CommandComplete,
    /// 履歴: 前のコマンド
    CommandHistoryPrev,
    /// 履歴: 次のコマンド
    CommandHistoryNext,

    // === ヘルプモード操作 ===
    /// ヘルプ画面スクロール
    HelpScroll(i32),

    // === ダイアログ ===
    /// ダイアログ表示
    ShowDialog(DialogState),
    /// ダイアログ確定
    DialogConfirm,
    /// ダイアログキャンセル
    DialogCancel,
    /// ダイアログ入力
    DialogInput(char),
    /// ダイアログ入力削除
    DialogBackspace,

    // === シェルコマンド ===
    /// 外部シェルコマンド実行
    ShellExecute(String),

    // === アプリ制御 ===
    /// アプリ終了
    Quit,
    /// 一時メッセージ表示
    ShowMessage(String, MessageLevel),
    /// 何もしない
    Noop,
}
```

### 2.2 イベントハンドラ (`event.rs`)

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::time::Duration;

/// イベントポーリング設定
const POLL_TIMEOUT: Duration = Duration::from_millis(50);
/// ダブルクリック判定閾値
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(500);

/// イベントを取得しActionに変換する
pub struct EventHandler;

impl EventHandler {
    /// 次のイベントをポーリングし、Actionに変換して返す
    pub fn next_action(app: &App) -> Result<Action, AppError> {
        if !event::poll(POLL_TIMEOUT)? {
            return Ok(Action::Noop);
        }

        let event = event::read()?;
        match event {
            Event::Key(key) => Self::handle_key(key, app),
            Event::Mouse(mouse) => Self::handle_mouse(mouse, app),
            Event::Resize(w, h) => Ok(Action::Refresh),
            _ => Ok(Action::Noop),
        }
    }

    /// キーイベントのハンドリング（モード別に振り分け）
    fn handle_key(key: KeyEvent, app: &App) -> Result<Action, AppError> {
        match app.mode {
            AppMode::Normal => Self::handle_normal_key(key, app),
            AppMode::Command => Self::handle_command_key(key, app),
            AppMode::Help => Self::handle_help_key(key),
            AppMode::Dialog => Self::handle_dialog_key(key, app),
        }
    }

    /// ノーマルモードのキーハンドリング
    fn handle_normal_key(key: KeyEvent, app: &App) -> Result<Action, AppError> {
        match key.code {
            // === vi ナビゲーション ===
            KeyCode::Char('h') => Ok(Action::ParentDirectory),
            KeyCode::Char('j') | KeyCode::Down => Ok(Action::MoveCursor(1)),
            KeyCode::Char('k') | KeyCode::Up => Ok(Action::MoveCursor(-1)),
            KeyCode::Char('l') | KeyCode::Enter => Ok(Action::Enter),
            KeyCode::Char('g') => Ok(Action::JumpTop),
            KeyCode::Char('G') => Ok(Action::JumpBottom),
            KeyCode::Char('~') => Ok(Action::GoHome),

            // === ペイン ===
            KeyCode::Tab => Ok(Action::SwitchPane),
            KeyCode::BackTab => Ok(Action::SwitchPane), // Shift+Tab

            // === ファイル操作 ===
            KeyCode::Char('c') | KeyCode::F(5) => Ok(Action::CopyFiles),
            KeyCode::Char('m') | KeyCode::F(6) => Ok(Action::MoveFiles),
            KeyCode::Char('d') | KeyCode::F(8) | KeyCode::Delete => Ok(Action::RequestDelete),
            KeyCode::Char('r') | KeyCode::F(2) => Ok(Action::RequestRename),
            KeyCode::Char('n') => Ok(Action::RequestCreateDir),
            KeyCode::Char('t') => Ok(Action::RequestCreateFile),

            // === マーキング ===
            KeyCode::Char(' ') => Ok(Action::ToggleMark),
            KeyCode::Char('a') => Ok(Action::ToggleMarkAll),

            // === 表示 ===
            KeyCode::Char('s') => Ok(Action::ShowSortMenu),
            KeyCode::Char('.') => Ok(Action::ToggleHidden),
            KeyCode::Char('i') => Ok(Action::ShowFileInfo),
            KeyCode::Char('/') => Ok(Action::ChangeDirectory(PathBuf::new())), // パス入力ダイアログ

            // === モード遷移 ===
            KeyCode::Char(':') => Ok(Action::EnterCommandMode),

            // === アプリ制御 ===
            KeyCode::Char('q') => Ok(Action::Quit),

            _ => Ok(Action::Noop),
        }
    }

    /// コマンドモードのキーハンドリング
    fn handle_command_key(key: KeyEvent, _app: &App) -> Result<Action, AppError> {
        match key.code {
            KeyCode::Esc => Ok(Action::ExitCommandMode),
            KeyCode::Enter => Ok(Action::CommandExecute),
            KeyCode::Backspace => Ok(Action::CommandBackspace),
            KeyCode::Tab => Ok(Action::CommandComplete),
            KeyCode::Up => Ok(Action::CommandHistoryPrev),
            KeyCode::Down => Ok(Action::CommandHistoryNext),
            KeyCode::Char(c) => Ok(Action::CommandInput(c)),
            _ => Ok(Action::Noop),
        }
    }

    /// ヘルプモードのキーハンドリング
    fn handle_help_key(key: KeyEvent) -> Result<Action, AppError> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(Action::ExitHelp),
            KeyCode::Char('j') | KeyCode::Down => Ok(Action::HelpScroll(1)),
            KeyCode::Char('k') | KeyCode::Up => Ok(Action::HelpScroll(-1)),
            KeyCode::Char('d') | KeyCode::PageDown => Ok(Action::HelpScroll(20)),
            KeyCode::Char('u') | KeyCode::PageUp => Ok(Action::HelpScroll(-20)),
            KeyCode::Char('g') => Ok(Action::HelpScroll(i32::MIN)), // 先頭へ
            KeyCode::Char('G') => Ok(Action::HelpScroll(i32::MAX)), // 末尾へ
            _ => Ok(Action::Noop),
        }
    }

    /// ダイアログモードのキーハンドリング
    fn handle_dialog_key(key: KeyEvent, app: &App) -> Result<Action, AppError> {
        match app.dialog.as_ref() {
            Some(DialogState::Confirm { .. }) => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => Ok(Action::DialogConfirm),
                KeyCode::Char('n') | KeyCode::Esc => Ok(Action::DialogCancel),
                _ => Ok(Action::Noop),
            },
            Some(DialogState::Input { .. }) => match key.code {
                KeyCode::Enter => Ok(Action::DialogConfirm),
                KeyCode::Esc => Ok(Action::DialogCancel),
                KeyCode::Backspace => Ok(Action::DialogBackspace),
                KeyCode::Char(c) => Ok(Action::DialogInput(c)),
                _ => Ok(Action::Noop),
            },
            Some(DialogState::Sort { .. }) => match key.code {
                KeyCode::Char('j') | KeyCode::Down => Ok(Action::MoveCursor(1)),
                KeyCode::Char('k') | KeyCode::Up => Ok(Action::MoveCursor(-1)),
                KeyCode::Enter | KeyCode::Char(' ') => Ok(Action::DialogConfirm),
                KeyCode::Esc => Ok(Action::DialogCancel),
                _ => Ok(Action::Noop),
            },
            _ => Ok(Action::Noop),
        }
    }

    /// マウスイベントのハンドリング
    fn handle_mouse(mouse: MouseEvent, app: &App) -> Result<Action, AppError> {
        match mouse.kind {
            MouseEventKind::Down(button) => {
                Self::handle_mouse_click(mouse.column, mouse.row, button, app)
            }
            MouseEventKind::ScrollUp => Ok(Action::MoveCursor(-3)),
            MouseEventKind::ScrollDown => Ok(Action::MoveCursor(3)),
            MouseEventKind::Drag(_) => {
                Self::handle_mouse_drag(mouse.column, mouse.row, app)
            }
            _ => Ok(Action::Noop),
        }
    }

    /// マウスクリック処理
    fn handle_mouse_click(
        x: u16, y: u16,
        button: crossterm::event::MouseButton,
        app: &App,
    ) -> Result<Action, AppError> {
        // クリック位置に応じてアクションを決定
        // 詳細は UI_DESIGN.md セクション7.1 に準拠
        todo!("implement based on hit-test regions")
    }

    /// マウスドラッグ処理
    fn handle_mouse_drag(x: u16, y: u16, app: &App) -> Result<Action, AppError> {
        if app.dragging_border {
            let terminal_width = /* get from app */ 80u16;
            let ratio = x as f32 / terminal_width as f32;
            Ok(Action::SetPaneRatio(ratio.clamp(0.2, 0.8)))
        } else {
            Ok(Action::Noop)
        }
    }
}
```

---

## 3. コマンドモジュール詳細

### 3.1 コマンドパーサー (`command/parser.rs`)

```rust
/// パース済みコマンド
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    /// コマンド名 (e.g., "help", "cd", "q")
    pub name: String,
    /// 引数リスト
    pub args: Vec<String>,
    /// 元の入力文字列
    pub raw: String,
}

/// シェルコマンド（:! プレフィックス）
#[derive(Debug, Clone)]
pub struct ShellCommand {
    pub command_line: String,
}

/// パース結果
#[derive(Debug, Clone)]
pub enum CommandParseResult {
    /// ビルトインコマンド
    Builtin(ParsedCommand),
    /// シェルコマンド (:! プレフィックス)
    Shell(ShellCommand),
    /// 空入力
    Empty,
    /// パースエラー
    Error(String),
}

pub struct CommandParser;

impl CommandParser {
    /// コマンド文字列をパースする
    ///
    /// # Examples
    /// - "help" → Builtin { name: "help", args: [] }
    /// - "help keybindings" → Builtin { name: "help", args: ["keybindings"] }
    /// - "cd ~/projects" → Builtin { name: "cd", args: ["~/projects"] }
    /// - "!ls -la" → Shell { command_line: "ls -la" }
    /// - "" → Empty
    pub fn parse(input: &str) -> CommandParseResult {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return CommandParseResult::Empty;
        }

        // シェルコマンド判定（:! で始まる）
        if let Some(shell_cmd) = trimmed.strip_prefix('!') {
            return CommandParseResult::Shell(ShellCommand {
                command_line: shell_cmd.trim().to_string(),
            });
        }

        // トークン分割（シンプルなスペース区切り）
        // TODO: クォート対応（パスにスペースが含まれる場合）
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();

        if tokens.is_empty() {
            return CommandParseResult::Empty;
        }

        let name = tokens[0].to_lowercase();
        let args: Vec<String> = tokens[1..].iter().map(|s| s.to_string()).collect();

        CommandParseResult::Builtin(ParsedCommand {
            name,
            args,
            raw: trimmed.to_string(),
        })
    }
}
```

### 3.2 コマンド実行エンジン (`command/executor.rs`)

```rust
/// ビルトインコマンドの定義
pub struct BuiltinCommand {
    /// コマンド名（プライマリ）
    pub name: &'static str,
    /// エイリアス
    pub aliases: &'static [&'static str],
    /// 説明文
    pub description: &'static str,
    /// 最小引数数
    pub min_args: usize,
    /// 最大引数数（usizeMAXは無制限）
    pub max_args: usize,
}

/// ビルトインコマンドレジストリ
pub const BUILTIN_COMMANDS: &[BuiltinCommand] = &[
    BuiltinCommand {
        name: "help",
        aliases: &[],
        description: "Show help screen",
        min_args: 0,
        max_args: 1,
    },
    BuiltinCommand {
        name: "quit",
        aliases: &["q"],
        description: "Quit application",
        min_args: 0,
        max_args: 0,
    },
    BuiltinCommand {
        name: "cd",
        aliases: &[],
        description: "Change directory",
        min_args: 1,
        max_args: 1,
    },
    BuiltinCommand {
        name: "sort",
        aliases: &[],
        description: "Change sort order (name/size/date/ext)",
        min_args: 1,
        max_args: 1,
    },
    BuiltinCommand {
        name: "set",
        aliases: &[],
        description: "Change setting",
        min_args: 1,
        max_args: 2,
    },
    BuiltinCommand {
        name: "mkdir",
        aliases: &[],
        description: "Create directory",
        min_args: 1,
        max_args: 1,
    },
    BuiltinCommand {
        name: "touch",
        aliases: &[],
        description: "Create file",
        min_args: 1,
        max_args: 1,
    },
    BuiltinCommand {
        name: "bookmark",
        aliases: &["bm"],
        description: "Add bookmark",
        min_args: 0,
        max_args: 1,
    },
];

pub struct CommandExecutor;

impl CommandExecutor {
    /// パース済みコマンドを実行し、Actionを返す
    pub fn execute(cmd: &ParsedCommand, app: &App) -> Result<Action, AppError> {
        // エイリアス解決
        let resolved_name = Self::resolve_alias(&cmd.name);

        // コマンド存在チェック
        let builtin = Self::find_builtin(&resolved_name)
            .ok_or_else(|| AppError::UnknownCommand(cmd.name.clone()))?;

        // 引数数チェック
        if cmd.args.len() < builtin.min_args || cmd.args.len() > builtin.max_args {
            return Err(AppError::InvalidArguments {
                command: cmd.name.clone(),
                expected: format!("{}-{}", builtin.min_args, builtin.max_args),
                got: cmd.args.len(),
            });
        }

        // コマンド別実行
        match resolved_name.as_str() {
            "help" => {
                let topic = cmd.args.first().map(|s| s.clone());
                Ok(Action::EnterHelp(topic))
            }
            "quit" => Ok(Action::Quit),
            "cd" => {
                let path = Self::resolve_path(&cmd.args[0], app)?;
                Ok(Action::ChangeDirectory(path))
            }
            "sort" => {
                let criteria = Self::parse_sort_criteria(&cmd.args[0])?;
                let mut order = app.active_pane_state().sort_order.clone();
                order.criteria = criteria;
                Ok(Action::SetSort(order))
            }
            "set" => Self::execute_set(&cmd.args, app),
            "mkdir" => Ok(Action::ExecuteCreateDir(cmd.args[0].clone())),
            "touch" => Ok(Action::ExecuteCreateFile(cmd.args[0].clone())),
            "bookmark" => {
                let name = cmd.args.first().map(|s| s.clone());
                Ok(Action::ShowMessage(
                    format!("Bookmarked: {}", app.active_pane_state().current_dir.display()),
                    MessageLevel::Success,
                ))
            }
            _ => Err(AppError::UnknownCommand(cmd.name.clone())),
        }
    }

    /// エイリアスを正式名に解決
    fn resolve_alias(name: &str) -> String {
        for cmd in BUILTIN_COMMANDS {
            if cmd.name == name || cmd.aliases.contains(&name) {
                return cmd.name.to_string();
            }
        }
        name.to_string()
    }

    /// ビルトインコマンドを名前で検索
    fn find_builtin(name: &str) -> Option<&'static BuiltinCommand> {
        BUILTIN_COMMANDS.iter().find(|c| c.name == name)
    }

    /// パス文字列を解決（~ 展開等）
    fn resolve_path(path_str: &str, app: &App) -> Result<PathBuf, AppError> {
        let expanded = if path_str.starts_with('~') {
            let home = dirs::home_dir().ok_or(AppError::HomeDirNotFound)?;
            home.join(path_str.strip_prefix("~/").unwrap_or(""))
        } else if path_str.starts_with('/') {
            PathBuf::from(path_str)
        } else {
            app.active_pane_state().current_dir.join(path_str)
        };

        let canonical = expanded.canonicalize()
            .map_err(|_| AppError::PathNotFound(expanded.clone()))?;

        if !canonical.is_dir() {
            return Err(AppError::NotADirectory(canonical));
        }

        Ok(canonical)
    }

    /// ソート基準文字列をパース
    fn parse_sort_criteria(s: &str) -> Result<SortCriteria, AppError> {
        match s.to_lowercase().as_str() {
            "name" | "n" => Ok(SortCriteria::Name),
            "size" | "s" => Ok(SortCriteria::Size),
            "date" | "d" | "time" | "modified" => Ok(SortCriteria::Date),
            "ext" | "e" | "extension" => Ok(SortCriteria::Extension),
            _ => Err(AppError::InvalidSortCriteria(s.to_string())),
        }
    }

    /// :set コマンドの実行
    fn execute_set(args: &[String], _app: &App) -> Result<Action, AppError> {
        match args[0].as_str() {
            "hidden" => Ok(Action::ToggleHidden),
            _ => Err(AppError::UnknownOption(args[0].clone())),
        }
    }
}
```

### 3.3 コマンド履歴 (`command/history.rs`)

```rust
use std::path::PathBuf;

const MAX_HISTORY_SIZE: usize = 500;
const HISTORY_FILE: &str = "history";

pub struct CommandHistory {
    /// 履歴エントリ（新しい順）
    entries: Vec<String>,
    /// 履歴ファイルのパス
    file_path: PathBuf,
}

impl CommandHistory {
    /// 履歴ファイルから読み込んで初期化
    pub fn load(config_dir: &PathBuf) -> Self {
        let file_path = config_dir.join(HISTORY_FILE);
        let entries = std::fs::read_to_string(&file_path)
            .unwrap_or_default()
            .lines()
            .rev()
            .take(MAX_HISTORY_SIZE)
            .map(|s| s.to_string())
            .collect();
        Self { entries, file_path }
    }

    /// コマンドを履歴に追加
    pub fn push(&mut self, command: &str) {
        let cmd = command.trim().to_string();
        if cmd.is_empty() {
            return;
        }
        // 重複を除去（直前と同じコマンドは追加しない）
        if self.entries.first().map(|s| s.as_str()) == Some(&cmd) {
            return;
        }
        self.entries.insert(0, cmd);
        if self.entries.len() > MAX_HISTORY_SIZE {
            self.entries.truncate(MAX_HISTORY_SIZE);
        }
    }

    /// インデックスで履歴を取得（0が最新）
    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|s| s.as_str())
    }

    /// 履歴をファイルに保存
    pub fn save(&self) -> Result<(), std::io::Error> {
        let content: String = self.entries.iter().rev().cloned().collect::<Vec<_>>().join("\n");
        std::fs::write(&self.file_path, content)
    }

    /// 履歴件数
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
```

### 3.4 オートコンプリート (`command/completer.rs`)

```rust
use std::path::Path;

pub struct Completer;

impl Completer {
    /// 入力文字列に対するオートコンプリート候補を生成
    pub fn complete(input: &str, current_dir: &Path) -> Vec<String> {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();

        match parts.len() {
            // コマンド名の補完
            1 => Self::complete_command(parts[0]),
            // 引数の補完
            2 => {
                let cmd = parts[0];
                let arg_prefix = parts[1];
                Self::complete_argument(cmd, arg_prefix, current_dir)
            }
            _ => vec![],
        }
    }

    /// コマンド名の補完
    fn complete_command(prefix: &str) -> Vec<String> {
        BUILTIN_COMMANDS
            .iter()
            .filter(|c| c.name.starts_with(prefix))
            .map(|c| c.name.to_string())
            .collect()
    }

    /// 引数の補完
    fn complete_argument(cmd: &str, prefix: &str, current_dir: &Path) -> Vec<String> {
        match cmd {
            "cd" | "mkdir" => Self::complete_path(prefix, current_dir, true),
            "touch" => Self::complete_path(prefix, current_dir, false),
            "sort" => Self::complete_from_list(
                prefix,
                &["name", "size", "date", "ext"],
            ),
            "set" => Self::complete_from_list(
                prefix,
                &["hidden"],
            ),
            "help" => Self::complete_from_list(
                prefix,
                &["navigation", "operations", "commands", "mouse", "config", "keybindings"],
            ),
            _ => vec![],
        }
    }

    /// パス補完
    fn complete_path(prefix: &str, current_dir: &Path, dirs_only: bool) -> Vec<String> {
        let (dir, file_prefix) = if prefix.contains('/') {
            let path = Path::new(prefix);
            let parent = path.parent().unwrap_or(Path::new("."));
            let file = path.file_name().map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            (current_dir.join(parent), file)
        } else {
            (current_dir.to_path_buf(), prefix.to_string())
        };

        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return vec![];
        };

        read_dir
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                if dirs_only {
                    entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                } else {
                    true
                }
            })
            .filter(|entry| {
                entry.file_name().to_string_lossy().starts_with(&file_prefix)
            })
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect()
    }

    /// 固定リストからの補完
    fn complete_from_list(prefix: &str, options: &[&str]) -> Vec<String> {
        options
            .iter()
            .filter(|opt| opt.starts_with(prefix))
            .map(|opt| opt.to_string())
            .collect()
    }
}
```

---

## 4. ファイル操作詳細

### 4.1 ファイル操作モジュール (`fs/operations.rs`)

```rust
use std::path::{Path, PathBuf};
use std::fs;

/// ファイル操作の進捗情報
pub struct Progress {
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub current_file: String,
    pub total_files: usize,
    pub completed_files: usize,
}

/// ファイル操作インターフェース
pub struct FileOperations;

impl FileOperations {
    /// ファイル/ディレクトリをコピー
    ///
    /// ディレクトリの場合は再帰的にコピー。
    /// 大きなファイルの場合は進捗コールバックを呼ぶ。
    pub fn copy(
        sources: &[PathBuf],
        dest_dir: &Path,
        on_progress: impl Fn(Progress),
        on_conflict: impl Fn(&Path) -> ConflictResolution,
    ) -> Result<usize, AppError> {
        let mut copied = 0;
        let total_bytes: u64 = sources.iter()
            .map(|s| Self::calculate_size(s))
            .sum();
        let mut current_bytes: u64 = 0;

        for source in sources {
            let dest = dest_dir.join(source.file_name().unwrap());

            // 同名ファイル存在チェック
            if dest.exists() {
                match on_conflict(&dest) {
                    ConflictResolution::Overwrite => {}
                    ConflictResolution::Skip => continue,
                    ConflictResolution::Cancel => return Ok(copied),
                }
            }

            if source.is_dir() {
                Self::copy_dir_recursive(source, &dest, &on_progress, &mut current_bytes, total_bytes)?;
            } else {
                Self::copy_file_with_progress(source, &dest, &on_progress, &mut current_bytes, total_bytes)?;
            }
            copied += 1;
        }

        Ok(copied)
    }

    /// ファイル/ディレクトリを移動
    ///
    /// 同一ファイルシステム内ならrename、跨ぐ場合はcopy+delete。
    pub fn move_files(
        sources: &[PathBuf],
        dest_dir: &Path,
        on_progress: impl Fn(Progress),
        on_conflict: impl Fn(&Path) -> ConflictResolution,
    ) -> Result<usize, AppError> {
        let mut moved = 0;

        for source in sources {
            let dest = dest_dir.join(source.file_name().unwrap());

            if dest.exists() {
                match on_conflict(&dest) {
                    ConflictResolution::Overwrite => {
                        if dest.is_dir() {
                            fs::remove_dir_all(&dest)?;
                        } else {
                            fs::remove_file(&dest)?;
                        }
                    }
                    ConflictResolution::Skip => continue,
                    ConflictResolution::Cancel => return Ok(moved),
                }
            }

            // まずrenameを試行（高速）
            match fs::rename(source, &dest) {
                Ok(()) => {}
                Err(_) => {
                    // 跨ぎ移動：copy + delete
                    Self::copy(&[source.clone()], dest_dir, &on_progress, &on_conflict)?;
                    if source.is_dir() {
                        fs::remove_dir_all(source)?;
                    } else {
                        fs::remove_file(source)?;
                    }
                }
            }
            moved += 1;
        }

        Ok(moved)
    }

    /// ファイル/ディレクトリを削除（ゴミ箱）
    pub fn delete_to_trash(paths: &[PathBuf]) -> Result<usize, AppError> {
        let mut deleted = 0;
        for path in paths {
            trash::delete(path)
                .map_err(|e| AppError::TrashError(e.to_string()))?;
            deleted += 1;
        }
        Ok(deleted)
    }

    /// ファイル/ディレクトリをリネーム
    pub fn rename(source: &Path, new_name: &str) -> Result<(), AppError> {
        let new_path = source.parent()
            .ok_or(AppError::InvalidPath)?
            .join(new_name);

        if new_path.exists() {
            return Err(AppError::FileAlreadyExists(new_path));
        }

        fs::rename(source, &new_path)?;
        Ok(())
    }

    /// ディレクトリ作成
    pub fn create_dir(parent: &Path, name: &str) -> Result<PathBuf, AppError> {
        let path = parent.join(name);
        if path.exists() {
            return Err(AppError::FileAlreadyExists(path));
        }
        fs::create_dir(&path)?;
        Ok(path)
    }

    /// ファイル作成
    pub fn create_file(parent: &Path, name: &str) -> Result<PathBuf, AppError> {
        let path = parent.join(name);
        if path.exists() {
            return Err(AppError::FileAlreadyExists(path));
        }
        fs::File::create(&path)?;
        Ok(path)
    }

    // --- 内部ヘルパー ---

    /// ファイル/ディレクトリの合計サイズを計算
    fn calculate_size(path: &Path) -> u64 {
        if path.is_file() {
            path.metadata().map(|m| m.len()).unwrap_or(0)
        } else {
            walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                .sum()
        }
    }

    /// ディレクトリの再帰コピー
    fn copy_dir_recursive(
        src: &Path,
        dest: &Path,
        on_progress: &impl Fn(Progress),
        current_bytes: &mut u64,
        total_bytes: u64,
    ) -> Result<(), AppError> {
        fs::create_dir_all(dest)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dest_path, on_progress, current_bytes, total_bytes)?;
            } else {
                Self::copy_file_with_progress(&src_path, &dest_path, on_progress, current_bytes, total_bytes)?;
            }
        }
        Ok(())
    }

    /// 進捗付きファイルコピー
    fn copy_file_with_progress(
        src: &Path,
        dest: &Path,
        on_progress: &impl Fn(Progress),
        current_bytes: &mut u64,
        total_bytes: u64,
    ) -> Result<(), AppError> {
        use std::io::{Read, Write, BufReader, BufWriter};

        const BUFFER_SIZE: usize = 64 * 1024; // 64KB

        let file_size = src.metadata()?.len();
        let reader = BufReader::new(fs::File::open(src)?);
        let writer = BufWriter::new(fs::File::create(dest)?);
        let mut reader = reader;
        let mut writer = writer;
        let mut buf = vec![0u8; BUFFER_SIZE];

        loop {
            let bytes_read = reader.read(&mut buf)?;
            if bytes_read == 0 {
                break;
            }
            writer.write_all(&buf[..bytes_read])?;
            *current_bytes += bytes_read as u64;

            on_progress(Progress {
                current_bytes: *current_bytes,
                total_bytes,
                current_file: src.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                total_files: 0,
                completed_files: 0,
            });
        }

        // パーミッションをコピー
        let perms = src.metadata()?.permissions();
        fs::set_permissions(dest, perms)?;

        Ok(())
    }
}

/// ファイル名衝突時の解決方法
#[derive(Debug, Clone, Copy)]
pub enum ConflictResolution {
    Overwrite,
    Skip,
    Cancel,
}
```

---

## 5. エラー型定義

### 5.1 アプリケーションエラー (`error.rs`)

```rust
use std::path::PathBuf;

/// アプリケーション全体のエラー型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // === I/O エラー ===
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    // === ファイル操作エラー ===
    #[error("File already exists: {0}")]
    FileAlreadyExists(PathBuf),

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("Invalid path")]
    InvalidPath,

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Trash operation failed: {0}")]
    TrashError(String),

    // === コマンドエラー ===
    #[error("Unknown command: '{0}'. Type :help for available commands.")]
    UnknownCommand(String),

    #[error("Invalid arguments for '{command}': expected {expected}, got {got}")]
    InvalidArguments {
        command: String,
        expected: String,
        got: usize,
    },

    #[error("Invalid sort criteria: '{0}'. Use: name, size, date, ext")]
    InvalidSortCriteria(String),

    #[error("Unknown option: '{0}'")]
    UnknownOption(String),

    // === 環境エラー ===
    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Terminal too small: minimum {min_w}x{min_h}, got {actual_w}x{actual_h}")]
    TerminalTooSmall {
        min_w: u16,
        min_h: u16,
        actual_w: u16,
        actual_h: u16,
    },

    // === 設定エラー ===
    #[error("Config parse error: {0}")]
    ConfigError(String),
}

impl AppError {
    /// ユーザー向けの短いエラーメッセージ
    pub fn user_message(&self) -> String {
        match self {
            AppError::UnknownCommand(cmd) => {
                format!("E: Unknown command '{}'. Type :help for available commands.", cmd)
            }
            AppError::PathNotFound(p) => {
                format!("E: Path not found: {}", p.display())
            }
            AppError::PermissionDenied(p) => {
                format!("E: Permission denied: {}", p.display())
            }
            AppError::FileAlreadyExists(p) => {
                format!("E: Already exists: {}", p.display())
            }
            _ => format!("E: {}", self),
        }
    }

    /// このエラーが致命的かどうか（致命的ならアプリ終了）
    pub fn is_fatal(&self) -> bool {
        matches!(self, AppError::TerminalTooSmall { .. })
    }
}
```

---

## 6. ダイアログ状態

### 6.1 ダイアログ型定義 (`ui/dialog.rs`)

```rust
/// ダイアログの状態
#[derive(Debug, Clone)]
pub enum DialogState {
    /// 確認ダイアログ（Yes/No）
    Confirm {
        title: String,
        message: String,
        detail: Option<String>,
        /// 現在のフォーカス（true=Yes, false=No）
        focus_yes: bool,
        /// 確定時に実行するアクション
        on_confirm: PendingAction,
    },
    /// 入力ダイアログ
    Input {
        title: String,
        label: String,
        value: String,
        cursor_pos: usize,
        /// 確定時に実行するアクション（入力値を引数に取る）
        on_submit: PendingInputAction,
    },
    /// ソート設定ダイアログ
    Sort {
        /// 選択中のインデックス
        selected: usize,
        /// 現在のソート設定
        current: SortOrder,
    },
    /// 進捗ダイアログ
    Progress {
        title: String,
        progress: Progress,
    },
}

/// 確認ダイアログの確定後アクション
#[derive(Debug, Clone)]
pub enum PendingAction {
    Delete(Vec<PathBuf>),
}

/// 入力ダイアログの確定後アクション
#[derive(Debug, Clone)]
pub enum PendingInputAction {
    Rename(PathBuf),
    CreateDir,
    CreateFile,
    PathInput,
}
```

---

## 7. UI描画詳細

### 7.1 描画エントリポイント (`ui/mod.rs`)

```rust
use ratatui::Frame;

/// メイン描画関数 - モードに応じて描画内容を切り替える
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 1. 基本レイアウトを計算
    let layout = Layout::calculate(area, app);

    // 2. 共通部分を描画（全モードで表示）
    header::render(frame, &layout.header, app);
    pane::render(frame, &layout.left_pane, &app.left_pane, app.active_pane == PaneSide::Left);
    pane::render(frame, &layout.right_pane, &app.right_pane, app.active_pane == PaneSide::Right);
    statusbar::render(frame, &layout.statusbar, app);

    // 3. モード別描画
    match app.mode {
        AppMode::Normal => {
            function_bar::render(frame, &layout.bottom_bar, app);
        }
        AppMode::Command => {
            command_line::render(frame, &layout.bottom_bar, &app.command_state);
        }
        AppMode::Help => {
            // ヘルプは全画面オーバーレイ
            help::render(frame, area, &app.help_state);
        }
        AppMode::Dialog => {}
    }

    // 4. ダイアログ（最前面に描画）
    if let Some(dialog) = &app.dialog {
        dialog::render(frame, area, dialog);
    }

    // 5. コンテキストメニュー（最前面）
    if let Some(menu) = &app.context_menu {
        context_menu::render(frame, menu);
    }

    // 6. 一時メッセージ（有効な場合、ステータスバーに上書き）
    if let Some(msg) = &app.message {
        if !msg.is_expired() {
            message::render(frame, &layout.statusbar, msg);
        }
    }
}
```

### 7.2 レイアウト計算 (`ui/layout.rs`)

```rust
use ratatui::layout::Rect;

/// 画面レイアウト（各領域のRect）
pub struct AppLayout {
    pub header: Rect,
    pub left_path: Rect,
    pub right_path: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    pub statusbar: Rect,
    pub bottom_bar: Rect,
}

impl AppLayout {
    /// ターミナルサイズとペイン比率からレイアウトを計算
    pub fn calculate(area: Rect, app: &App) -> Self {
        let border_x = (area.width as f32 * app.pane_ratio) as u16;
        let border_x = border_x.clamp(20, area.width.saturating_sub(20));

        Self {
            header: Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: 1,
            },
            left_path: Rect {
                x: area.x,
                y: area.y + 1,
                width: border_x,
                height: 1,
            },
            right_path: Rect {
                x: area.x + border_x,
                y: area.y + 1,
                width: area.width - border_x,
                height: 1,
            },
            left_pane: Rect {
                x: area.x,
                y: area.y + 2,
                width: border_x,
                height: area.height.saturating_sub(4),
            },
            right_pane: Rect {
                x: area.x + border_x,
                y: area.y + 2,
                width: area.width - border_x,
                height: area.height.saturating_sub(4),
            },
            statusbar: Rect {
                x: area.x,
                y: area.height.saturating_sub(2),
                width: area.width,
                height: 1,
            },
            bottom_bar: Rect {
                x: area.x,
                y: area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            },
        }
    }

    /// ペイン境界のx座標
    pub fn border_x(&self) -> u16 {
        self.left_pane.width
    }
}
```

---

## 8. メインエントリポイント

### 8.1 `main.rs`

```rust
use std::io;
use crossterm::{
    event::{EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod app;
mod command;
mod config;
mod error;
mod event;
mod fs;
mod mode;
mod ui;
mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // パニックハンドラの設置
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // ターミナルを復元してからパニック情報を出力
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // CLI引数パース
    let args = cli::parse_args();

    // 設定読み込み
    let config = config::Config::load(&args)?;

    // ターミナル初期化
    let mut terminal = setup_terminal()?;

    // アプリケーション実行
    let result = run_app(&mut terminal, config, args);

    // ターミナル復元（必ず実行）
    restore_terminal()?;

    // エラーがあれば表示
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

/// ターミナルをTUIモードに初期化
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, AppError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// ターミナルを通常モードに復元
fn restore_terminal() -> Result<(), AppError> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

/// メインアプリケーションループ
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: config::Config,
    args: cli::Args,
) -> Result<(), AppError> {
    let mut app = App::new(config, args)?;

    loop {
        // 描画
        terminal.draw(|frame| ui::render(frame, &app))?;

        // イベント処理
        let action = event::EventHandler::next_action(&app)?;

        // アクション適用
        app.apply_action(action)?;

        // 期限切れメッセージの削除
        app.cleanup_expired_messages();

        // 終了判定
        if app.should_quit {
            // コマンド履歴を保存
            app.command_state.history.save()?;
            break;
        }
    }

    Ok(())
}
```

---

## 9. 設定型定義

### 9.1 設定構造体 (`config/mod.rs`)

```rust
use serde::Deserialize;

/// アプリケーション設定
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub keybindings: KeyBindingsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_false")]
    pub show_hidden: bool,
    #[serde(default = "default_sort")]
    pub default_sort: String,
    #[serde(default = "default_true")]
    pub sort_ascending: bool,
    #[serde(default = "default_true")]
    pub dirs_first: bool,
    #[serde(default = "default_true")]
    pub confirm_delete: bool,
    #[serde(default = "default_true")]
    pub use_trash: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_pane_ratio")]
    pub pane_ratio: f32,
    #[serde(default = "default_scroll_margin")]
    pub scroll_margin: usize,
    #[serde(default = "default_date_format")]
    pub date_format: String,
    #[serde(default = "default_time_format")]
    pub time_format: String,
    #[serde(default = "default_true")]
    pub show_welcome: bool,
    #[serde(default = "default_true")]
    pub mouse_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeyBindingsConfig {
    // カスタムキーバインド（省略時はデフォルト）
    // key: action のマッピング
}

// --- デフォルト値関数 ---
fn default_false() -> bool { false }
fn default_true() -> bool { true }
fn default_sort() -> String { "name".to_string() }
fn default_pane_ratio() -> f32 { 0.5 }
fn default_scroll_margin() -> usize { 3 }
fn default_date_format() -> String { "%m/%d".to_string() }
fn default_time_format() -> String { "%H:%M".to_string() }
fn default_theme() -> String { "default".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            theme: ThemeConfig::default(),
            keybindings: KeyBindingsConfig::default(),
        }
    }
}

impl Config {
    /// 設定ファイルを読み込む。ファイルがなければデフォルトを返す。
    pub fn load(args: &cli::Args) -> Result<Self, AppError> {
        let config_path = args.config.clone()
            .unwrap_or_else(|| {
                dirs::config_dir()
                    .unwrap_or_default()
                    .join("mdir")
                    .join("config.toml")
            });

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| AppError::ConfigError(e.to_string()))?;
            toml::from_str(&content)
                .map_err(|e| AppError::ConfigError(e.to_string()))
        } else {
            Ok(Config::default())
        }
    }
}
```

---

## 10. ユーティリティ

### 10.1 フォーマット (`utils/format.rs`)

```rust
use std::time::SystemTime;
use chrono::{DateTime, Local, Datelike, Timelike};

/// ファイルサイズを人間可読形式にフォーマット
///
/// # Examples
/// - 0 → "0B"
/// - 512 → "512B"
/// - 1024 → "1.0KB"
/// - 1536 → "1.5KB"
/// - 1048576 → "1.0MB"
/// - 1073741824 → "1.0GB"
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{}B", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

/// SystemTimeを表示用文字列にフォーマット
///
/// 当日の場合: "HH:MM"
/// それ以外: "MM/DD"
/// 年が違う場合: "YYYY"
pub fn format_time(time: SystemTime, date_fmt: &str, time_fmt: &str) -> String {
    let datetime: DateTime<Local> = time.into();
    let now = Local::now();

    if datetime.date_naive() == now.date_naive() {
        datetime.format(time_fmt).to_string()
    } else if datetime.year() == now.year() {
        datetime.format(date_fmt).to_string()
    } else {
        datetime.format("%Y").to_string()
    }
}

/// パスを表示用に短縮
///
/// ホームディレクトリを ~ に、長いパスを ... で省略
pub fn format_path(path: &std::path::Path, max_width: usize) -> String {
    let home = dirs::home_dir();
    let display = if let Some(home) = &home {
        if let Ok(relative) = path.strip_prefix(home) {
            format!("~/{}", relative.display())
        } else {
            path.display().to_string()
        }
    } else {
        path.display().to_string()
    };

    if display.len() <= max_width {
        display
    } else {
        let truncated = &display[display.len() - (max_width - 4)..];
        format!(".../{}", truncated.trim_start_matches('/'))
    }
}
```

---

## 11. テスト設計

### 11.1 単体テスト例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // --- format_size テスト ---
    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0B");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_048_576), "1.0MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1_073_741_824), "1.0GB");
    }

    // --- コマンドパーサーテスト ---
    #[test]
    fn test_parse_help_no_args() {
        let result = CommandParser::parse("help");
        match result {
            CommandParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "help");
                assert!(cmd.args.is_empty());
            }
            _ => panic!("Expected Builtin"),
        }
    }

    #[test]
    fn test_parse_help_with_topic() {
        let result = CommandParser::parse("help keybindings");
        match result {
            CommandParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "help");
                assert_eq!(cmd.args, vec!["keybindings"]);
            }
            _ => panic!("Expected Builtin"),
        }
    }

    #[test]
    fn test_parse_shell_command() {
        let result = CommandParser::parse("!ls -la");
        match result {
            CommandParseResult::Shell(cmd) => {
                assert_eq!(cmd.command_line, "ls -la");
            }
            _ => panic!("Expected Shell"),
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = CommandParser::parse("");
        assert!(matches!(result, CommandParseResult::Empty));
    }

    #[test]
    fn test_parse_cd_with_tilde() {
        let result = CommandParser::parse("cd ~/projects");
        match result {
            CommandParseResult::Builtin(cmd) => {
                assert_eq!(cmd.name, "cd");
                assert_eq!(cmd.args, vec!["~/projects"]);
            }
            _ => panic!("Expected Builtin"),
        }
    }

    // --- ソートテスト ---
    #[test]
    fn test_sort_by_name_ascending() {
        let mut entries = vec![
            make_test_entry("charlie", EntryType::File),
            make_test_entry("alpha", EntryType::File),
            make_test_entry("bravo", EntryType::File),
        ];
        let order = SortOrder {
            criteria: SortCriteria::Name,
            ascending: true,
            dirs_first: false,
        };
        order.sort(&mut entries);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn test_sort_dirs_first() {
        let mut entries = vec![
            make_test_entry("file_b", EntryType::File),
            make_test_entry("dir_a", EntryType::Directory),
            make_test_entry("file_a", EntryType::File),
        ];
        let order = SortOrder::default(); // dirs_first = true
        order.sort(&mut entries);
        assert_eq!(entries[0].name, "dir_a");
    }

    // --- モード遷移テスト ---
    #[test]
    fn test_normal_can_enter_command() {
        assert!(AppMode::Normal.can_enter_command());
        assert!(!AppMode::Help.can_enter_command());
        assert!(!AppMode::Dialog.can_enter_command());
    }

    // --- ヘルパー ---
    fn make_test_entry(name: &str, entry_type: EntryType) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            entry_type,
            size: 0,
            modified: SystemTime::now(),
            created: None,
            mode: 0o644,
            is_hidden: false,
            is_symlink: false,
            symlink_target: None,
            extension: None,
        }
    }
}
```

### 11.2 統合テスト例

```rust
// tests/integration/file_operations.rs

use std::fs;
use tempfile::TempDir;

#[test]
fn test_copy_file() {
    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    // テストファイル作成
    let src_file = src_dir.path().join("test.txt");
    fs::write(&src_file, "hello world").unwrap();

    // コピー実行
    let result = FileOperations::copy(
        &[src_file.clone()],
        dest_dir.path(),
        |_| {},
        |_| ConflictResolution::Overwrite,
    );

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // コピー先の確認
    let dest_file = dest_dir.path().join("test.txt");
    assert!(dest_file.exists());
    assert_eq!(fs::read_to_string(&dest_file).unwrap(), "hello world");

    // 元ファイルも残っている
    assert!(src_file.exists());
}

#[test]
fn test_move_file() {
    let src_dir = TempDir::new().unwrap();
    let dest_dir = TempDir::new().unwrap();

    let src_file = src_dir.path().join("test.txt");
    fs::write(&src_file, "hello world").unwrap();

    let result = FileOperations::move_files(
        &[src_file.clone()],
        dest_dir.path(),
        |_| {},
        |_| ConflictResolution::Overwrite,
    );

    assert!(result.is_ok());
    let dest_file = dest_dir.path().join("test.txt");
    assert!(dest_file.exists());
    assert!(!src_file.exists()); // 元ファイルは削除されている
}

#[test]
fn test_create_dir() {
    let parent = TempDir::new().unwrap();
    let result = FileOperations::create_dir(parent.path(), "new_dir");
    assert!(result.is_ok());
    assert!(parent.path().join("new_dir").is_dir());
}

#[test]
fn test_create_dir_already_exists() {
    let parent = TempDir::new().unwrap();
    fs::create_dir(parent.path().join("existing")).unwrap();
    let result = FileOperations::create_dir(parent.path(), "existing");
    assert!(matches!(result, Err(AppError::FileAlreadyExists(_))));
}

#[test]
fn test_rename_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("old.txt");
    fs::write(&file, "content").unwrap();

    let result = FileOperations::rename(&file, "new.txt");
    assert!(result.is_ok());
    assert!(!dir.path().join("old.txt").exists());
    assert!(dir.path().join("new.txt").exists());
}
```

---

## 12. 依存クレート一覧（Cargo.toml）

```toml
[package]
name = "mdir"
version = "0.1.0"
edition = "2021"
authors = ["mdir contributors"]
description = "A vi-style terminal file manager inspired by MDIR"
license = "MIT"
repository = "https://github.com/xxx/mdir"

[dependencies]
ratatui = "0.29"          # TUIフレームワーク
crossterm = "0.28"        # ターミナルバックエンド
tokio = { version = "1", features = ["full"] }  # 非同期ランタイム
serde = { version = "1", features = ["derive"] } # シリアライズ
toml = "0.8"              # TOML設定ファイル
walkdir = "2"             # ディレクトリ走査
trash = "5"               # ゴミ箱操作
chrono = "0.4"            # 日時処理
clap = { version = "4", features = ["derive"] }  # CLI引数
dirs = "5"                # 標準ディレクトリ
thiserror = "2"           # エラー型マクロ
tracing = "0.1"           # ログ
tracing-appender = "0.2"  # ファイルログ

[dev-dependencies]
tempfile = "3"            # テスト用一時ファイル
insta = "1"               # スナップショットテスト
```

---

*本ドキュメントは開発の進行に応じて更新される。*
