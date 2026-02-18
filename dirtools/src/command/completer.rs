// command/completer.rs - コマンドオートコンプリート
//
// Tab キー押下時にコマンド名やパスの補完候補を生成する。
// コマンド名は BUILTIN_NAMES から、パスは実ファイルシステムから候補を取得。

use std::path::Path;

/// ビルトインコマンド名一覧
const BUILTIN_NAMES: &[&str] = &[
    "help", "quit", "q", "cd", "sort", "set", "mkdir", "touch", "bookmark", "find",
];

/// ヘルプトピック一覧
const HELP_TOPICS: &[&str] = &[
    "navigation",
    "operations",
    "commands",
    "mouse",
    "config",
    "keybindings",
];

/// ソート基準一覧
const SORT_OPTIONS: &[&str] = &["name", "size", "date", "ext"];

/// 入力文字列に対するオートコンプリート候補を生成
pub fn complete(input: &str, current_dir: &Path) -> Vec<String> {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();

    match parts.len() {
        // コマンド名の補完
        1 => complete_from_list(parts[0], BUILTIN_NAMES),
        // 引数の補完
        2 => {
            let cmd = parts[0];
            let prefix = parts[1];
            complete_argument(cmd, prefix, current_dir)
        }
        _ => vec![],
    }
}

/// コマンド引数の補完
fn complete_argument(cmd: &str, prefix: &str, current_dir: &Path) -> Vec<String> {
    match cmd {
        "cd" | "mkdir" => complete_path(prefix, current_dir, true),
        "touch" => complete_path(prefix, current_dir, false),
        "sort" => complete_from_list(prefix, SORT_OPTIONS),
        "set" => complete_from_list(prefix, &["hidden"]),
        "help" => complete_from_list(prefix, HELP_TOPICS),
        _ => vec![],
    }
}

/// 固定リストからプレフィックスマッチで補完
fn complete_from_list(prefix: &str, options: &[&str]) -> Vec<String> {
    options
        .iter()
        .filter(|opt| opt.starts_with(prefix))
        .map(|opt| opt.to_string())
        .collect()
}

/// ファイルシステムからパス補完
fn complete_path(prefix: &str, current_dir: &Path, dirs_only: bool) -> Vec<String> {
    let (dir, file_prefix) = if prefix.contains('/') {
        let path = Path::new(prefix);
        let parent = path.parent().unwrap_or(Path::new("."));
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        // ~ 展開
        let resolved = if let Some(rest) = parent.to_string_lossy().strip_prefix('~') {
            dirs::home_dir()
                .unwrap_or_default()
                .join(rest.trim_start_matches('/'))
        } else if parent.is_absolute() {
            parent.to_path_buf()
        } else {
            current_dir.join(parent)
        };
        (resolved, file)
    } else {
        (current_dir.to_path_buf(), prefix.to_string())
    };

    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return vec![];
    };

    read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            if dirs_only {
                e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
            } else {
                true
            }
        })
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&file_prefix)
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect()
}
