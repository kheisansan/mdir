# mdir

vi スタイルのキーボード操作を軸にした、macOS 向けターミナルファイルマネージャー。
1990年代に韓国で広く使われた DOS 用ファイルマネージャー「MDIR」にインスパイアされています。

![Rust](https://img.shields.io/badge/Rust-2021-orange)
![License](https://img.shields.io/badge/License-MIT-blue)

## 特徴

- **2ペイン構成** — 左右にディレクトリを表示し、ファイルのコピー・移動が直感的
- **vi ライクなキー操作** — `h/j/k/l` でナビゲーション、`:` でコマンドモード
- **日本語 IME 対応** — IME がオンでも操作可能（JIS かな入力にも対応）
- **マウス対応** — クリック・ダブルクリック・スクロールに対応
- **Git 連携** — リポジトリ内ではブランチ名を表示、`y` キーで git pull を実行
- **ゴミ箱削除** — ファイル削除はゴミ箱経由で安全に実行
- **設定ファイル** — `~/.config/mdir/config.toml` でカスタマイズ可能

## インストール

### ソースからビルド

```bash
git clone <repository-url>
cd mdir/dirtools
cargo clean && cargo build --release
```

ビルド後のバイナリは `target/release/mdir` に生成されます。

**重要**: 改修を反映したバイナリを実行するには、必ず上記のとおり `cargo clean` してから `cargo build --release` すること。  
実行するときは **プロジェクト内のバイナリを直接指定**すること（PATH の `mdir` は古い可能性があります）:

```bash
# プロジェクト内のバイナリを実行（推奨）
./target/release/mdir --version   # 0.1.1 と表示されれば改修版
./target/release/mdir
```

PATH にインストールして使う場合は、ビルド後に次で更新:

```bash
cargo install --path .
```

### 実行

```bash
# カレントディレクトリで起動
./target/release/mdir
# または
cargo run

# 左右ペインのディレクトリを指定して起動
./target/release/mdir --left ~/Documents --right ~/Downloads
```

## キーバインド

### ナビゲーション

| キー | 動作 |
|------|------|
| `h` / `←` / `BS` | 親ディレクトリへ移動 |
| `j` / `↓` | カーソルを下に移動 |
| `k` / `↑` | カーソルを上に移動 |
| `l` / `→` / `Enter` | ディレクトリに入る / ファイルを開く |
| `g` / `Home` | 先頭へジャンプ |
| `G` / `End` | 末尾へジャンプ |
| `PgUp` / `PgDn` | ページ単位スクロール |
| `~` | ホームディレクトリへ移動 |
| `Tab` | アクティブペインを切り替え |

### ファイル操作

| キー | 動作 |
|------|------|
| `c` / `F5` | 対向ペインへコピー |
| `m` / `F6` | 対向ペインへ移動 |
| `d` / `F8` / `Del` | 削除（ゴミ箱へ移動） |
| `r` / `F2` | 名前変更 |
| `n` / `F7` | 新規フォルダ作成 |
| `t` / `F4` | 新規ファイル作成 |
| `x` | パーミッション変更（chmod） |
| `Space` / `Ins` | マーク / マーク解除 |
| `a` | 全選択 / 全解除 |
| `s` | ソートメニュー |
| `.` | 隠しファイル表示切り替え |
| `i` | ファイル情報 |
| `y` | Git Pull（Git リポジトリ内のみ） |

### コマンドモード

`:` を入力するとコマンドモードに入ります。

| コマンド | 動作 |
|----------|------|
| `:help` | ヘルプ画面を表示 |
| `:q` / `:quit` | 終了 |
| `:cd <パス>` | ディレクトリ移動 |
| `:sort <基準>` | ソート（name / size / date / ext） |
| `:find <文字列>` | ファイル名検索（`o`/`p` で候補移動） |
| `:mkdir <名前>` | ディレクトリ作成 |
| `:touch <名前>` | ファイル作成 |
| `:!<コマンド>` | シェルコマンド実行 |

### 検索結果ナビゲーション

| キー | 動作 |
|------|------|
| `o` | 前の候補へ |
| `p` | 次の候補へ |

## 設定

設定ファイルは `~/.config/mdir/config.toml` に配置します。
初回起動時に自動生成されます。

```toml
[general]
show_hidden = false

[ui]
pane_ratio = 0.5
date_format = "%Y-%m-%d"
time_format = "%H:%M"
show_welcome = true
```

## プロジェクト構成

```
src/
├── main.rs          # エントリポイント
├── app.rs           # アプリケーション状態・アクション処理
├── mode.rs          # モード管理（Normal / Command / Help / Dialog）
├── event.rs         # キーボード・マウスイベント処理
├── error.rs         # エラー型定義
├── ime.rs           # IME 制御（macOS）
├── fs/
│   ├── mod.rs       # ペイン状態管理
│   ├── entry.rs     # ファイルエントリモデル・ソート
│   └── operations.rs # ファイル操作（コピー・移動・削除等）
├── ui/
│   ├── mod.rs       # メイン描画
│   ├── pane.rs      # ファイルペイン描画
│   ├── statusbar.rs # ステータスバー描画
│   ├── function_bar.rs # ファンクションバー描画
│   ├── dialog.rs    # ダイアログ描画
│   ├── help.rs      # ヘルプ画面描画
│   ├── header.rs    # ヘッダーバー描画
│   ├── command_line.rs # コマンドライン描画
│   ├── layout.rs    # レイアウト計算
│   ├── theme.rs     # カラーテーマ定義
│   └── welcome.rs   # ウェルカム画面描画
├── command/
│   ├── mod.rs       # コマンド状態
│   ├── parser.rs    # コマンド解析
│   ├── executor.rs  # コマンド実行
│   ├── history.rs   # コマンド履歴
│   └── completer.rs # コマンド補完
├── config/
│   └── mod.rs       # 設定管理
└── utils/
    ├── mod.rs
    ├── format.rs    # フォーマットユーティリティ
    └── git.rs       # Git ユーティリティ
```

## 動作要件

- macOS（Apple Silicon / Intel）
- Rust 1.70+
- ターミナルエミュレータ（Terminal.app / iTerm2 / Alacritty 等）

## ライセンス

MIT
