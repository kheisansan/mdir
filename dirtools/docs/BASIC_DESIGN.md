# mdir - 基本設計書

**バージョン:** 0.1.0
**作成日:** 2026-02-11
**対応要件:** REQUIREMENTS.md
**対応UI設計:** docs/UI_DESIGN.md
**ステータス:** ドラフト

---

## 1. システムアーキテクチャ

### 1.1 アーキテクチャ概要

本アプリケーションはレイヤードアーキテクチャを採用し、以下の4層で構成する。

```
┌─────────────────────────────────────────────────────────────┐
│                   Presentation Layer                         │
│          (UI描画 / ウィジェット / レイアウト)                   │
├─────────────────────────────────────────────────────────────┤
│                   Application Layer                          │
│      (イベント処理 / モード管理 / コマンド実行)                 │
├─────────────────────────────────────────────────────────────┤
│                     Domain Layer                             │
│    (ファイルエントリ / ソート / フィルタ / マーキング)            │
├─────────────────────────────────────────────────────────────┤
│                  Infrastructure Layer                         │
│   (ファイルシステム / OS連携 / 設定 / ターミナル制御)            │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 レイヤー間の依存ルール

- 上位レイヤーは直下のレイヤーにのみ依存する（飛び越し依存禁止）
- 下位レイヤーは上位レイヤーに依存しない
- Domain Layer は外部クレートに依存しない（純粋なRustコード）
- Infrastructure Layer のみがOSやファイルシステムに直接アクセスする

```
Presentation ──► Application ──► Domain ──► (std only)
                      │                         ▲
                      └──► Infrastructure ──────┘
```

### 1.3 レイヤー責務一覧

| レイヤー | 責務 | 主要モジュール |
|---|---|---|
| **Presentation** | TUI描画、ウィジェット、レイアウト管理、テーマ | `ui/*` |
| **Application** | イベントルーティング、モード状態管理、コマンド実行、アプリ状態 | `app.rs`, `event.rs`, `mode.rs`, `command/*` |
| **Domain** | ファイルエントリモデル、ソート/フィルタロジック、マーキング | `fs/entry.rs`, ソート/フィルタ関連 |
| **Infrastructure** | ファイルI/O、ゴミ箱連携、設定ファイル読み書き、ターミナル制御 | `fs/operations.rs`, `fs/trash.rs`, `config/*`, `utils/*` |

---

## 2. モジュール構成

### 2.1 モジュール依存関係図

```
main.rs
  │
  ├──► app::App                      ← アプリケーションのルート
  │      │
  │      ├──► mode::AppMode          ← モード状態機械
  │      │
  │      ├──► event::EventHandler    ← イベント受信・振り分け
  │      │      │
  │      │      └──► crossterm       ← ターミナルイベント取得
  │      │
  │      ├──► command::Commander     ← コマンド実行エンジン
  │      │      ├──► command::Parser
  │      │      ├──► command::History
  │      │      └──► command::Completer
  │      │
  │      ├──► fs::PaneState (x2)    ← 左右ペインの状態
  │      │      ├──► fs::Entry
  │      │      ├──► fs::SortOrder
  │      │      └──► fs::Filter
  │      │
  │      ├──► fs::Operations         ← ファイル操作実行
  │      │      └──► fs::Trash
  │      │
  │      └──► config::Config         ← 設定管理
  │             ├──► config::KeyBindings
  │             └──► config::Theme
  │
  └──► ui::render()                  ← 描画エントリポイント
         ├──► ui::layout
         ├──► ui::header
         ├──► ui::pane
         ├──► ui::statusbar
         ├──► ui::command_line
         ├──► ui::context_menu
         ├──► ui::dialog
         ├──► ui::help
         └──► ui::theme
```

### 2.2 モジュール一覧と責務

| モジュール | ファイル | 責務 |
|---|---|---|
| `main` | `main.rs` | エントリポイント、ターミナル初期化/復元、メインループ起動 |
| `app` | `app.rs` | アプリケーション全体の状態保持、イベントディスパッチ、描画トリガー |
| `event` | `event.rs` | crossterm イベントのポーリング、キーボード/マウスイベントの抽象化 |
| `mode` | `mode.rs` | Normal/Command/Help モードの状態機械、遷移ルール |
| `command` | `command/mod.rs` | コマンドモジュールの公開インターフェース |
| `command::parser` | `command/parser.rs` | コマンド文字列の解析、引数分離 |
| `command::executor` | `command/executor.rs` | 解析済みコマンドの実行、各アクションへのディスパッチ |
| `command::history` | `command/history.rs` | コマンド入力履歴の保持、上下キーでの履歴参照 |
| `command::completer` | `command/completer.rs` | コマンド名・パスのオートコンプリート候補生成 |
| `ui` | `ui/mod.rs` | UI描画のエントリポイント、モードに応じた描画振り分け |
| `ui::layout` | `ui/layout.rs` | 画面領域の分割計算、リサイズ対応 |
| `ui::header` | `ui/header.rs` | ヘッダーバーの描画 |
| `ui::pane` | `ui/pane.rs` | ファイルペインの描画（パスバー含む） |
| `ui::statusbar` | `ui/statusbar.rs` | ステータスバーの描画 |
| `ui::command_line` | `ui/command_line.rs` | コマンドモード時の入力行描画 |
| `ui::context_menu` | `ui/context_menu.rs` | 右クリックコンテキストメニューの描画 |
| `ui::dialog` | `ui/dialog.rs` | 確認/入力/進捗ダイアログの描画 |
| `ui::help` | `ui/help.rs` | ヘルプ画面の描画 |
| `ui::theme` | `ui/theme.rs` | カラーテーマ定義、スタイル生成 |
| `fs` | `fs/mod.rs` | ファイルシステムモジュールの公開インターフェース |
| `fs::entry` | `fs/entry.rs` | ファイル/ディレクトリエントリのデータモデル |
| `fs::operations` | `fs/operations.rs` | コピー/移動/削除/リネーム等の実行 |
| `fs::watcher` | `fs/watcher.rs` | ディレクトリの変更監視、自動リフレッシュ |
| `fs::trash` | `fs/trash.rs` | macOSゴミ箱への移動処理 |
| `config` | `config/mod.rs` | 設定管理の公開インターフェース |
| `config::keybindings` | `config/keybindings.rs` | キーバインドの定義と設定読み込み |
| `config::theme` | `config/theme.rs` | カラーテーマの定義と設定読み込み |
| `utils::format` | `utils/format.rs` | ファイルサイズ・日時のフォーマット |
| `utils::platform` | `utils/platform.rs` | macOS固有処理の抽象化 |

---

## 3. 状態管理設計

### 3.1 アプリケーション状態構造

```
App
├── mode: AppMode              ← 現在のモード（Normal/Command/Help/Dialog）
├── left_pane: PaneState       ← 左ペインの状態
├── right_pane: PaneState      ← 右ペインの状態
├── active_pane: PaneSide      ← アクティブペイン（Left/Right）
├── command_state: CommandState ← コマンドモードの入力状態
├── dialog_state: Option<DialogState> ← ダイアログ表示状態
├── help_state: HelpState      ← ヘルプ画面の状態
├── pane_ratio: f32            ← ペイン幅比率（0.0-1.0）
├── config: Config             ← ユーザー設定
├── message: Option<Message>   ← 一時メッセージ（エラー等）
├── should_quit: bool          ← 終了フラグ
└── first_launch: bool         ← 初回起動フラグ
```

### 3.2 ペイン状態

```
PaneState
├── current_dir: PathBuf         ← カレントディレクトリ
├── entries: Vec<FileEntry>      ← ファイルエントリ一覧
├── cursor_index: usize          ← カーソル位置
├── scroll_offset: usize         ← スクロールオフセット
├── marked_indices: HashSet<usize> ← マーク済みインデックス
├── sort_order: SortOrder        ← ソート設定
├── show_hidden: bool            ← 隠しファイル表示
└── filter: Option<String>       ← フィルタ文字列
```

### 3.3 コマンド状態

```
CommandState
├── input: String                ← 現在の入力文字列
├── cursor_pos: usize            ← カーソル位置
├── history: CommandHistory       ← コマンド履歴
├── history_index: Option<usize> ← 履歴参照中のインデックス
└── completions: Vec<String>     ← オートコンプリート候補
```

---

## 4. イベント処理フロー

### 4.1 メインループ

```
┌─────────────────────────────────────────────────┐
│                  Main Loop                       │
│                                                   │
│  loop {                                          │
│    1. イベント取得 (crossterm poll, 50ms timeout) │
│    2. イベント処理 (モードに応じたハンドリング)      │
│    3. 状態更新                                    │
│    4. UI描画 (ratatui render)                     │
│    5. should_quit チェック → break               │
│  }                                               │
└─────────────────────────────────────────────────┘
```

### 4.2 イベントディスパッチフロー

```
crossterm::Event
       │
       ▼
  ┌──────────┐
  │ EventType │
  ├──────────┤
  │ Key?     ├──► handle_key_event(key)
  │ Mouse?   ├──► handle_mouse_event(mouse)
  │ Resize?  ├──► handle_resize(w, h)
  └──────────┘
       │
       ▼
  ┌───────────────┐
  │ Current Mode? │
  ├───────────────┤
  │ Normal    ├──► normal_mode_handler(event)
  │ Command   ├──► command_mode_handler(event)
  │ Help      ├──► help_mode_handler(event)
  │ Dialog    ├──► dialog_handler(event)
  └───────────────┘
       │
       ▼
  Action (状態変更の指示)
       │
       ▼
  App.apply_action(action) → 状態更新
```

### 4.3 アクション一覧

イベントハンドラは直接状態を変更せず、`Action` を返す。`App` がアクションを適用する。

| カテゴリ | アクション | 内容 |
|---|---|---|
| **ナビゲーション** | `MoveCursor(Direction)` | カーソル移動 |
| | `EnterDirectory` | ディレクトリに入る |
| | `ParentDirectory` | 親ディレクトリへ |
| | `GoHome` | ホームディレクトリへ |
| | `JumpTop` | 先頭へ |
| | `JumpBottom` | 末尾へ |
| **ペイン** | `SwitchPane` | アクティブペイン切り替え |
| | `SetActivePane(PaneSide)` | ペイン指定切り替え |
| | `ResizePane(f32)` | ペイン幅変更 |
| **ファイル操作** | `Copy` | コピー |
| | `Move` | 移動 |
| | `Delete` | 削除 |
| | `Rename(String)` | リネーム |
| | `CreateDir(String)` | ディレクトリ作成 |
| | `CreateFile(String)` | ファイル作成 |
| **マーキング** | `ToggleMark` | マーク切替 |
| | `ToggleMarkAll` | 全選択切替 |
| **表示** | `ToggleHidden` | 隠しファイル切替 |
| | `SetSort(SortOrder)` | ソート変更 |
| | `ShowFileInfo` | ファイル情報表示 |
| **モード** | `EnterCommandMode` | コマンドモードへ |
| | `ExitCommandMode` | ノーマルモードへ |
| | `EnterHelpMode` | ヘルプモードへ |
| | `ExitHelpMode` | ノーマルモードへ |
| **コマンド** | `CommandInput(char)` | コマンド文字入力 |
| | `CommandBackspace` | コマンド文字削除 |
| | `CommandExecute` | コマンド実行 |
| | `CommandComplete` | オートコンプリート |
| | `CommandHistoryUp` | 履歴: 前へ |
| | `CommandHistoryDown` | 履歴: 次へ |
| **ダイアログ** | `ShowDialog(DialogType)` | ダイアログ表示 |
| | `DialogConfirm` | ダイアログ確定 |
| | `DialogCancel` | ダイアログキャンセル |
| **アプリ** | `Quit` | アプリ終了 |
| | `Refresh` | 表示リフレッシュ |
| | `ShowMessage(Message)` | メッセージ表示 |

---

## 5. モード状態機械

### 5.1 状態遷移図

```
                        ┌─────────────────────────────────────────────┐
                        │                                              │
           ┌────────────┼─────────────────────────┐                   │
           │            │                          │                   │
           ▼            │                          ▼                   │
    ┌────────────┐  ':' │   ┌──────────────┐  ':help'+Enter   ┌──────────┐
    │            │──────┘   │              │──────────────────►│          │
    │   Normal   │─────────►│   Command    │                   │   Help   │
    │            │  ':'     │              │                   │          │
    │            │◄─────────│              │                   │          │
    └────────────┘ Esc/     └──────────────┘                   └──────────┘
       ▲   │      Back/                                           │
       │   │      Enter                                           │
       │   │                                                      │
       │   │  delete/rename等                                     │ q/Esc
       │   ▼                                                      │
    ┌────────────┐                                                │
    │            │                                                │
    │   Dialog   │────────────────────────────────────────────────┘
    │            │  Confirm/Cancel
    │            │──────────────────► Normal に戻る
    └────────────┘
```

### 5.2 モード遷移ルール

| 現在のモード | イベント | 遷移先 | 条件 |
|---|---|---|---|
| Normal | `:` キー | Command | — |
| Normal | `d`/`Delete` キー（削除対象あり） | Dialog (Confirm) | マーク or カーソル上にファイルあり |
| Normal | `r` キー | Dialog (Input) | カーソル上にファイルあり |
| Normal | `n` キー | Dialog (Input) | — |
| Command | `Esc` | Normal | — |
| Command | `[Back]` クリック | Normal | — |
| Command | `Enter` (`:help`) | Help | コマンドが `help` |
| Command | `Enter` (その他) | Normal | コマンド実行後 |
| Command | `Enter` (エラー) | Normal | エラーメッセージ表示後2秒 |
| Help | `q` / `Esc` | Normal | — |
| Dialog | `y` / `Enter`(Yes) | Normal | 操作実行後 |
| Dialog | `n` / `Esc` | Normal | — |

---

## 6. データモデル

### 6.1 ファイルエントリ (FileEntry)

アプリ内で扱うファイル/ディレクトリの情報を保持するモデル。

| フィールド | 型 | 内容 |
|---|---|---|
| `name` | `String` | ファイル名 |
| `path` | `PathBuf` | 絶対パス |
| `entry_type` | `EntryType` | ファイル種別 |
| `size` | `u64` | バイトサイズ |
| `modified` | `SystemTime` | 最終更新日時 |
| `created` | `Option<SystemTime>` | 作成日時 |
| `permissions` | `Permissions` | Unixパーミッション |
| `is_hidden` | `bool` | 隠しファイルかどうか |
| `is_symlink` | `bool` | シンボリックリンクかどうか |
| `symlink_target` | `Option<PathBuf>` | リンク先パス |
| `git_status` | `Option<GitStatus>` | Git状態（Phase 3） |

### 6.2 列挙型

#### EntryType
| バリアント | 内容 |
|---|---|
| `Directory` | ディレクトリ |
| `File` | 通常ファイル |
| `Executable` | 実行可能ファイル |
| `Symlink` | シンボリックリンク |

#### SortOrder
| フィールド | 型 | 内容 |
|---|---|---|
| `criteria` | `SortCriteria` | ソート基準 (Name/Size/Date/Extension) |
| `ascending` | `bool` | 昇順/降順 |
| `dirs_first` | `bool` | ディレクトリ優先 |

#### AppMode
| バリアント | 内容 |
|---|---|
| `Normal` | 通常モード |
| `Command` | コマンド入力モード |
| `Help` | ヘルプ表示モード |
| `Dialog(DialogType)` | ダイアログ表示中 |

#### DialogType
| バリアント | 内容 |
|---|---|
| `Confirm { title, message, on_confirm }` | 確認ダイアログ |
| `Input { title, initial_value, on_submit }` | 入力ダイアログ |
| `Sort` | ソート設定ダイアログ |
| `Progress { title, current, total }` | 進捗ダイアログ |

---

## 7. コマンド処理設計

### 7.1 コマンド処理パイプライン

```
ユーザー入力 ":help keybindings"
       │
       ▼
  ┌──────────────┐
  │  Tokenizer   │  "help", "keybindings" にトークン分割
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │   Parser     │  Command { name: "help", args: ["keybindings"] } に構造化
  └──────┬───────┘
         │
         ▼
  ┌──────────────┐
  │  Validator   │  コマンド存在チェック、引数バリデーション
  └──────┬───────┘
         │ OK / Err
         ▼
  ┌──────────────┐
  │  Executor    │  対応するActionを生成・実行
  └──────┬───────┘
         │
         ▼
  Action::EnterHelpMode(topic: Some("keybindings"))
```

### 7.2 ビルトインコマンド設計

| コマンド | 引数 | バリデーション | 生成するAction |
|---|---|---|---|
| `help` | `[topic]` (省略可) | トピック名が有効か | `EnterHelpMode(topic)` |
| `q` / `quit` | なし | — | `Quit` |
| `cd` | `<path>` (必須) | パスが存在するか | `ChangeDirectory(path)` |
| `sort` | `<criteria>` (必須) | name/size/date/ext のいずれか | `SetSort(order)` |
| `set` | `<option> [value]` | 有効な設定名か | `SetOption(key, value)` |
| `!` | `<command>` (必須) | — | `ShellExecute(command)` |
| `mkdir` | `<name>` (必須) | 既存でないか | `CreateDir(name)` |
| `touch` | `<name>` (必須) | — | `CreateFile(name)` |
| `bookmark` | `[name]` (省略可) | — | `AddBookmark(name)` |

### 7.3 オートコンプリート

`Tab` キー押下時の補完ルール:

| 入力状態 | 補完対象 |
|---|---|
| `:` の直後（コマンド名入力中） | ビルトインコマンド名 |
| `:cd ` の後 | ディレクトリパス |
| `:sort ` の後 | ソート基準名 (name/size/date/ext) |
| `:set ` の後 | 設定キー名 |
| `:help ` の後 | ヘルプトピック名 |
| `:!` の後 | シェルコマンド（$PATH から） |

---

## 8. ファイル操作設計

### 8.1 操作対象の決定ルール

```
操作対象の決定:
  1. マーク済みファイルが存在する場合 → マーク済みファイル全てが対象
  2. マーク済みファイルがない場合 → カーソル位置のファイルが対象
```

### 8.2 各操作のフロー

#### コピー (Copy)
```
1. 操作対象ファイルを決定
2. 対向ペインのパスを取得（コピー先）
3. 同名ファイルが存在する場合 → 上書き確認ダイアログ
4. コピー実行（大きなファイルは進捗ダイアログ付き）
5. 対向ペインのファイル一覧をリフレッシュ
6. ステータスバーに完了メッセージ
```

#### 移動 (Move)
```
1. 操作対象ファイルを決定
2. 対向ペインのパスを取得（移動先）
3. 同名ファイルが存在する場合 → 上書き確認ダイアログ
4. 移動実行
5. 両ペインのファイル一覧をリフレッシュ
6. ステータスバーに完了メッセージ
```

#### 削除 (Delete)
```
1. 操作対象ファイルを決定
2. 確認ダイアログ表示（「ゴミ箱に移動しますか？」）
3. ゴミ箱に移動（trash クレート使用）
4. カレントペインのファイル一覧をリフレッシュ
5. カーソル位置を調整（削除後にはみ出る場合）
6. ステータスバーに完了メッセージ
```

#### リネーム (Rename)
```
1. カーソル位置のファイルを取得
2. 入力ダイアログ表示（現在のファイル名を初期値にセット）
3. 新しい名前のバリデーション（空文字、既存名、不正文字チェック）
4. リネーム実行
5. ファイル一覧をリフレッシュ
6. リネーム後のファイルにカーソルを移動
```

### 8.3 エラー時の振る舞い

| エラー | ユーザーへの通知 | 復帰 |
|---|---|---|
| 権限不足 | ステータスバーにエラーメッセージ（Red） | 操作をスキップ |
| ディスク容量不足 | エラーダイアログ表示 | 操作を中断 |
| ファイルが存在しない（操作中に削除された） | ステータスバーにエラーメッセージ | リフレッシュ |
| コピー先に同名ファイル | 上書き確認ダイアログ | ユーザー選択 |

---

## 9. 設定管理設計

### 9.1 設定ファイルの場所

```
~/.config/mdir/
├── config.toml          ← メイン設定ファイル
├── bookmarks.toml       ← ブックマーク
└── history              ← コマンド履歴（プレーンテキスト）
```

### 9.2 設定ファイル構造 (config.toml)

```toml
# mdir configuration

[general]
show_hidden = false          # 隠しファイルの初期表示
default_sort = "name"        # デフォルトソート (name/size/date/ext)
sort_ascending = true        # ソート方向
dirs_first = true            # ディレクトリ優先表示
confirm_delete = true        # 削除確認ダイアログ
use_trash = true             # ゴミ箱を使用（falseで完全削除）

[ui]
pane_ratio = 0.5             # ペイン幅比率
scroll_margin = 3            # スクロール余白（行数）
date_format = "%m/%d"        # 日付フォーマット
time_format = "%H:%M"        # 時刻フォーマット
show_welcome = true          # ウェルカムメッセージ表示

[theme]
name = "default"             # テーマ名（default / カスタム名）

[keybindings]
# キーバインドのカスタマイズ（省略時はデフォルト）
# 例: quit = "Q"
```

### 9.3 設定読み込みフロー

```
1. デフォルト設定を構築 (Config::default())
2. ~/.config/mdir/config.toml が存在するか確認
   a. 存在する → ファイルを読み込み、デフォルトにマージ
   b. 存在しない → デフォルト設定をそのまま使用、初回起動フラグON
3. 設定のバリデーション（不正値はデフォルトにフォールバック）
4. Config を App にセット
```

---

## 10. エラーハンドリング方針

### 10.1 エラー分類

| カテゴリ | 例 | 対応 |
|---|---|---|
| **致命的エラー** | ターミナル初期化失敗 | ターミナル復元後、stderrにメッセージ出力して終了 |
| **操作エラー** | ファイルコピー失敗、権限不足 | ステータスバーにエラーメッセージ表示、操作を中断 |
| **入力エラー** | 不正なコマンド、存在しないパス | コマンドラインにエラーメッセージ表示 |
| **警告** | ターミナルサイズが小さい | 警告オーバーレイ表示 |

### 10.2 パニックハンドラ

アプリケーションがパニックした場合でも、ターミナルを正常状態に復元するカスタムパニックハンドラを設置する。

```
パニック発生
  │
  ├─► ターミナルのraw modeを解除
  ├─► 代替スクリーンバッファから復帰
  ├─► マウスキャプチャを解除
  ├─► カーソルを表示
  └─► パニックメッセージをstderrに出力
```

---

## 11. テスト戦略

### 11.1 テスト階層

| テスト種別 | 対象 | ツール |
|---|---|---|
| **単体テスト** | Domain層のロジック（ソート、フィルタ、フォーマット等） | `#[cfg(test)]` モジュール内テスト |
| **結合テスト** | ファイル操作、コマンドパーサー | `tests/` ディレクトリ、tempdir使用 |
| **スナップショットテスト** | UI描画結果 | `insta` クレート + ratatui `TestBackend` |

### 11.2 テスト対象優先度

| 優先度 | 対象 | 理由 |
|---|---|---|
| **高** | ファイル操作 (operations.rs) | データ破損リスク |
| **高** | コマンドパーサー (parser.rs) | ユーザー入力の正確な解釈 |
| **高** | モード遷移 (mode.rs) | アプリの基本動作 |
| **中** | ソート/フィルタ | 表示の正確性 |
| **中** | フォーマット (format.rs) | 表示の正確性 |
| **低** | UI描画 | スナップショットテストで担保 |

---

## 12. 外部インターフェース

### 12.1 CLIインターフェース

```
mdir [OPTIONS] [PATH]

Arguments:
  [PATH]  起動時に表示するディレクトリ（省略時: カレントディレクトリ）

Options:
  -l, --left <PATH>     左ペインの初期ディレクトリ
  -r, --right <PATH>    右ペインの初期ディレクトリ
  -c, --config <FILE>   設定ファイルのパス
  --no-mouse            マウスサポートを無効化
  -v, --version         バージョン表示
  -h, --help            ヘルプ表示
```

### 12.2 終了コード

| コード | 意味 |
|---|---|
| 0 | 正常終了 |
| 1 | 一般エラー |
| 2 | 設定ファイルエラー |

---

## 13. ログ戦略

- 通常実行時: ログ出力なし（TUIを阻害しないため）
- デバッグ実行時: `MDIR_LOG=debug mdir` で `~/.local/share/mdir/mdir.log` にファイルログ出力
- ログレベル: `error`, `warn`, `info`, `debug`, `trace`
- ログライブラリ: `tracing` クレート

---

*本ドキュメントは開発の進行に応じて更新される。*
