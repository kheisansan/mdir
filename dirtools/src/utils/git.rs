// utils/git.rs - Git ユーティリティ
//
// ディレクトリが Git リポジトリ内かどうかの判定、
// 現在のブランチ名の取得、git pull の実行などを提供する。
//
// ブランチ名の取得は .git/HEAD ファイルを直接読む方式を採用。
// サブプロセスを起動しないため、TUI の raw mode でも安全かつ高速に動作する。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// .git ディレクトリのパスを探す（指定ディレクトリから親方向に辿る）
///
/// 通常のリポジトリでは `.git` はディレクトリだが、
/// サブモジュールやワークツリーでは `.git` がファイル（gitdir 参照）の場合がある。
fn find_git_dir(dir: &Path) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    loop {
        let git_path = current.join(".git");
        if git_path.is_dir() {
            return Some(git_path);
        }
        // .git がファイルの場合（サブモジュール/ワークツリー）
        if git_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&git_path) {
                let content = content.trim();
                if let Some(gitdir) = content.strip_prefix("gitdir: ") {
                    let resolved = if Path::new(gitdir).is_absolute() {
                        PathBuf::from(gitdir)
                    } else {
                        current.join(gitdir)
                    };
                    if resolved.is_dir() {
                        return Some(resolved);
                    }
                }
            }
        }
        if !current.pop() {
            return None;
        }
    }
}

/// 指定ディレクトリが Git リポジトリ内にあるかどうかを判定する
pub fn is_git_repo(dir: &Path) -> bool {
    find_git_dir(dir).is_some()
}

/// 現在チェックアウトしているブランチ名を取得する
///
/// `.git/HEAD` ファイルを直接読み取る方式で、サブプロセスを起動しない。
/// - `ref: refs/heads/<branch>` → ブランチ名を返す
/// - ハッシュ値のみ → 先頭7文字を `(<hash>)` 形式で返す
pub fn current_branch(dir: &Path) -> Option<String> {
    let git_dir = find_git_dir(dir)?;
    let head_path = git_dir.join("HEAD");
    let content = std::fs::read_to_string(head_path).ok()?;
    let content = content.trim();

    if let Some(ref_path) = content.strip_prefix("ref: refs/heads/") {
        Some(ref_path.to_string())
    } else if content.len() >= 7 && content.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("({})", &content[..7]))
    } else {
        None
    }
}

/// ブランチ情報（表示名・現在フラグ・checkout に渡す引数）
pub struct GitBranchItem {
    /// リスト表示用（ローカルはそのまま、リモートは "origin/xxx"）
    pub display: String,
    pub is_current: bool,
    /// git checkout に渡す引数（リモートは "origin/xxx" からブランチ名のみ）
    pub checkout_arg: String,
}

/// 指定ディレクトリで `git branch -a` を実行し、ローカル・リモート全ブランチを返す
///
/// 現在のブランチには `is_current = true`。リモートは display を "origin/xxx" にし、
/// checkout_arg はローカルにチェックアウトするときの名前（例: "feature/x"）にする。
pub fn git_branch_list(dir: &Path) -> Vec<GitBranchItem> {
    match Command::new("git")
        .args(["branch", "-a"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut items = Vec::new();
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let is_current = line.starts_with('*');
                let rest = line.trim_start_matches('*').trim();

                // "remotes/origin/HEAD -> origin/main" はスキップ
                if rest.starts_with("remotes/") && rest.contains("HEAD ->") {
                    continue;
                }

                if let Some(remote) = rest.strip_prefix("remotes/origin/") {
                    let display = format!("origin/{}", remote);
                    let checkout_arg = remote.to_string();
                    items.push(GitBranchItem {
                        display,
                        is_current,
                        checkout_arg,
                    });
                } else {
                    items.push(GitBranchItem {
                        display: rest.to_string(),
                        is_current,
                        checkout_arg: rest.to_string(),
                    });
                }
            }
            items
        }
        Err(_) => Vec::new(),
    }
}

/// `git checkout` の実行結果
pub struct GitCheckoutResult {
    pub success: bool,
    pub message: String,
}

/// 指定ディレクトリで `git checkout <args>` を実行する
///
/// args は空白分割してそのまま git checkout の引数として渡す。
/// 例: "main" → `git checkout main`
///      "-b feature/xxx" → `git checkout -b feature/xxx`
pub fn git_checkout(dir: &Path, args: &str) -> GitCheckoutResult {
    let checkout_args: Vec<&str> = args.split_whitespace().collect();
    let mut cmd_args = vec!["checkout"];
    cmd_args.extend(checkout_args.iter());

    match Command::new("git")
        .args(&cmd_args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.trim().is_empty() {
                result.push_str(stdout.trim());
            }
            if !stderr.trim().is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(stderr.trim());
            }
            if result.is_empty() {
                result = "完了".to_string();
            }
            GitCheckoutResult {
                success: output.status.success(),
                message: result,
            }
        }
        Err(e) => GitCheckoutResult {
            success: false,
            message: format!("git checkout 実行エラー: {}", e),
        },
    }
}

/// 指定ディレクトリで `git pull` を実行し、stdout + stderr を返す
pub fn git_pull(dir: &Path) -> String {
    match Command::new("git")
        .args(["pull"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.trim().is_empty() {
                result.push_str(stdout.trim());
            }
            if !stderr.trim().is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(stderr.trim());
            }
            if result.is_empty() {
                "完了".to_string()
            } else {
                result
            }
        }
        Err(e) => format!("git pull 実行エラー: {}", e),
    }
}
