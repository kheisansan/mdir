// main.rs - mdir エントリポイント
//
// ターミナルの初期化/復元、メインループの実行、
// パニックハンドラの設置を行う。
// ターミナル復元は異常終了時にも確実に行うことで、
// ユーザーのシェルを壊さないことを最優先に設計する。

mod app;
mod command;
mod config;
mod error;
mod event;
mod fs;
mod ime;
mod mode;
mod ui;
mod utils;

use app::App;
use clap::Parser;
use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use error::AppError;
use std::io::{self, Write};
use std::path::PathBuf;

/// mdir - vi スタイルのターミナルファイルマネージャー
#[derive(Parser)]
#[command(name = "mdir", version = "0.1.1", about = "MDIR 風の vi スタイル ターミナルファイルマネージャー")]
struct Cli {
    /// 開くディレクトリ（省略時はカレントディレクトリ）
    #[arg(value_name = "パス")]
    path: Option<PathBuf>,

    /// 左ペインの初期ディレクトリ
    #[arg(short = 'l', long = "left")]
    left: Option<PathBuf>,

    /// 右ペインの初期ディレクトリ
    #[arg(short = 'r', long = "right")]
    right: Option<PathBuf>,

    /// 設定ファイルのパス
    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,

    /// マウスサポートを無効にする
    #[arg(long = "no-mouse")]
    no_mouse: bool,
}

fn main() {
    // パニック時にもターミナルを復元するカスタムハンドラ
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    // 実行してエラーがあればターミナル復元後に表示
    if let Err(e) = run() {
        eprintln!("mdir: {}", e);
        std::process::exit(1);
    }
}

/// メインの実行関数
fn run() -> Result<(), AppError> {
    let cli = Cli::parse();

    // 設定読み込み
    let mut config = config::Config::load(cli.config.as_ref())?;
    if cli.no_mouse {
        config.ui.mouse_enabled = false;
    }

    // 初期ディレクトリ決定
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    let left_path = cli.left.or(cli.path.clone()).unwrap_or_else(|| cwd.clone());
    let right_path = cli.right.unwrap_or(cwd);

    // ターミナル初期化
    let mut terminal = setup_terminal(config.ui.mouse_enabled)?;

    // IME を ASCII モードに強制（日本語入力が必要な場面では一時的に解除される）
    ime::init();

    // アプリケーション初期化
    let mut app = App::new(config, left_path, right_path)?;

    // メインループ
    let result = run_main_loop(&mut terminal, &mut app);

    // IME を起動前の状態に復元
    ime::cleanup();

    // Terminal オブジェクトを明示的にドロップしてから復元
    // （Terminal が stdout を保持しているため、先に解放する）
    drop(terminal);

    // ターミナル復元（常に実行）
    restore_terminal()?;

    result
}

/// ターミナルを TUI モードに初期化
fn setup_terminal(
    mouse_enabled: bool,
) -> Result<ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>, AppError> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if mouse_enabled {
        execute!(stdout, EnableMouseCapture)?;
    }
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = ratatui::Terminal::new(backend)?;
    Ok(terminal)
}

/// ターミナルを通常モードに復元
fn restore_terminal() -> Result<(), AppError> {
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        Show
    )?;
    // stdout をフラッシュして確実に復元コマンドが送信されるようにする
    stdout.flush()?;
    Ok(())
}

/// メインループ
fn run_main_loop(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), AppError> {
    loop {
        // 描画
        terminal.draw(|frame| ui::render(frame, app))?;

        // イベント取得 → Action 変換
        let action = event::next_action(app)?;

        // Action 適用（エラーはメッセージとして表示）
        if let Err(e) = app.apply_action(action) {
            app.message = Some(app::TimedMessage::new(
                e.user_message(),
                app::MessageLevel::Error,
            ));
        }

        // 期限切れメッセージ削除
        app.cleanup_expired_messages();

        // 外部変更の自動検出＆リフレッシュ
        app.check_external_changes();

        // 終了判定
        if app.should_quit {
            break;
        }
    }

    Ok(())
}
