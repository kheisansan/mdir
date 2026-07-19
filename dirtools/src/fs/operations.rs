// fs/operations.rs - ファイル操作（コピー/移動/削除/リネーム/作成）
//
// 全てのファイル操作をこのモジュールに集約し、
// エラーハンドリングと進捗通知を統一的に行う。

use crate::error::AppError;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// この合計サイズ（バイト）以上のコピー/移動は、同期処理でUIをブロックする代わりに
/// バックグラウンドスレッド + 進捗ダイアログで実行する。
pub const LARGE_COPY_THRESHOLD_BYTES: u64 = 50 * 1024 * 1024; // 50MB

/// 進捗報告つきコピーの読み書きチャンクサイズ
const COPY_CHUNK_SIZE: usize = 1024 * 1024; // 1MB

/// ファイル名衝突時の解決方法
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConflictResolution {
    Overwrite,
    Skip,
    Cancel,
    /// リネームしてコピー/移動（新しいパスを指定）
    Rename(PathBuf),
}

/// ファイル操作の進捗情報（大きなファイルコピー時等に使用）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Progress {
    pub current_bytes: u64,
    pub total_bytes: u64,
    pub current_file: String,
}

/// ファイル/ディレクトリをコピー
///
/// ディレクトリの場合は再帰コピー。大きなファイルは進捗コールバックで通知。
pub fn copy_files(
    sources: &[PathBuf],
    dest_dir: &Path,
    on_conflict: &dyn Fn(&Path) -> ConflictResolution,
) -> Result<usize, AppError> {
    let mut copied = 0;

    for source in sources {
        let file_name = source
            .file_name()
            .ok_or(AppError::InvalidPath)?;
        let dest = dest_dir.join(file_name);

        // 同名ファイル存在チェック
        let actual_dest = if dest.exists() {
            match on_conflict(&dest) {
                ConflictResolution::Overwrite => dest.clone(),
                ConflictResolution::Skip => continue,
                ConflictResolution::Cancel => return Ok(copied),
                ConflictResolution::Rename(ref new_path) => new_path.clone(),
            }
        } else {
            dest.clone()
        };

        if source.is_dir() {
            copy_dir_recursive(source, &actual_dest)?;
        } else {
            fs::copy(source, &actual_dest)?;
        }
        copied += 1;
    }

    Ok(copied)
}

/// ファイル/ディレクトリを移動
///
/// 同一 FS 内なら rename、跨ぐ場合は copy + delete にフォールバック。
pub fn move_files(
    sources: &[PathBuf],
    dest_dir: &Path,
    on_conflict: &dyn Fn(&Path) -> ConflictResolution,
) -> Result<usize, AppError> {
    let mut moved = 0;

    for source in sources {
        let file_name = source
            .file_name()
            .ok_or(AppError::InvalidPath)?;
        let dest = dest_dir.join(file_name);

        let actual_dest = if dest.exists() {
            match on_conflict(&dest) {
                ConflictResolution::Overwrite => {
                    if dest.is_dir() {
                        fs::remove_dir_all(&dest)?;
                    } else {
                        fs::remove_file(&dest)?;
                    }
                    dest.clone()
                }
                ConflictResolution::Skip => continue,
                ConflictResolution::Cancel => return Ok(moved),
                ConflictResolution::Rename(ref new_path) => new_path.clone(),
            }
        } else {
            dest.clone()
        };

        // まず rename を試行（高速、同一FS内の場合）
        if fs::rename(source, &actual_dest).is_err() {
            // 別 FS の場合: copy + delete
            if source.is_dir() {
                copy_dir_recursive(source, &actual_dest)?;
                fs::remove_dir_all(source)?;
            } else {
                fs::copy(source, &actual_dest)?;
                fs::remove_file(source)?;
            }
        }
        moved += 1;
    }

    Ok(moved)
}

/// ファイル/ディレクトリを削除（OS のゴミ箱へ移動）
/// macOS: Trash、Windows: ごみ箱、Linux: freedesktop trash
pub fn delete_to_trash(paths: &[PathBuf]) -> Result<usize, AppError> {
    let mut deleted = 0;
    for path in paths {
        trash::delete(path).map_err(|e| AppError::TrashError(e.to_string()))?;
        deleted += 1;
    }
    Ok(deleted)
}

/// ファイル/ディレクトリをリネーム
pub fn rename(source: &Path, new_name: &str) -> Result<(), AppError> {
    let parent = source.parent().ok_or(AppError::InvalidPath)?;
    let new_path = parent.join(new_name);

    if new_path.exists() {
        return Err(AppError::FileAlreadyExists(new_path));
    }

    fs::rename(source, &new_path)?;
    Ok(())
}

/// ディレクトリを作成
pub fn create_dir(parent: &Path, name: &str) -> Result<PathBuf, AppError> {
    let path = parent.join(name);
    if path.exists() {
        return Err(AppError::FileAlreadyExists(path));
    }
    fs::create_dir(&path)?;
    Ok(path)
}

/// 空ファイルを作成
pub fn create_file(parent: &Path, name: &str) -> Result<PathBuf, AppError> {
    let path = parent.join(name);
    if path.exists() {
        return Err(AppError::FileAlreadyExists(path));
    }
    fs::File::create(&path)?;
    Ok(path)
}

/// パーミッション変更（Unix のみ有効）
///
/// 8進数モード（例: 0o755）を指定してファイル/ディレクトリのパーミッションを変更する。
/// Windows では no-op（0 件変更として返す）。
#[cfg(unix)]
pub fn chmod(paths: &[PathBuf], mode: u32) -> Result<usize, AppError> {
    use std::os::unix::fs::PermissionsExt;
    let mut count = 0;
    for path in paths {
        let perms = fs::Permissions::from_mode(mode);
        fs::set_permissions(path, perms)?;
        count += 1;
    }
    Ok(count)
}

/// パーミッション変更（Windows スタブ）
#[cfg(not(unix))]
pub fn chmod(_paths: &[PathBuf], _mode: u32) -> Result<usize, AppError> {
    Ok(0)
}

// ---------------------------------------------------------------------------
// 進捗報告つきコピー/移動（大きいファイル操作をバックグラウンドスレッドで実行する用）
// ---------------------------------------------------------------------------
//
// 通常の copy_files / move_files は fs::copy / fs::rename による高速パスを使い、
// メインループを止めて同期的に実行される（小さいファイルなら瞬時に終わるため）。
// 合計サイズが LARGE_COPY_THRESHOLD_BYTES 以上の場合は、代わりにこちらの
// *_with_progress 版をバックグラウンドスレッドから呼び出す。
// on_progress は (今回コピーしたバイト数, ファイル名) を受け取り、続行するなら true、
// キャンセルするなら false を返す。

/// 指定パス群（ファイル・ディレクトリ）の合計サイズをバイト単位で計算する。
/// 進捗バー表示要否（LARGE_COPY_THRESHOLD_BYTES との比較）に使う。
/// 読み取れないエントリは 0 として扱う（エラーで処理全体を止めない）。
pub fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|p| path_size(p)).sum()
}

fn path_size(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::read_dir(path)
            .map(|entries| entries.flatten().map(|e| path_size(&e.path())).sum())
            .unwrap_or(0),
        Ok(meta) => meta.len(),
        Err(_) => 0,
    }
}

/// ファイルをチャンク単位でコピーし、書き込むたびに `on_progress` に通知する。
/// キャンセルされたら `Ok(true)` を返す（呼び出し側で以降の処理を打ち切る）。
fn copy_file_with_progress(
    src: &Path,
    dest: &Path,
    on_progress: &dyn Fn(u64, &str) -> bool,
) -> Result<bool, AppError> {
    let file_name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut reader = fs::File::open(src)?;
    let mut writer = fs::File::create(dest)?;
    let mut buf = vec![0u8; COPY_CHUNK_SIZE];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        if !on_progress(n as u64, &file_name) {
            return Ok(true);
        }
    }
    if let Ok(meta) = fs::metadata(src) {
        let _ = fs::set_permissions(dest, meta.permissions());
    }
    Ok(false)
}

/// ディレクトリを再帰的にコピー（進捗報告つき）。キャンセルされたら `Ok(true)`。
fn copy_dir_recursive_with_progress(
    src: &Path,
    dest: &Path,
    on_progress: &dyn Fn(u64, &str) -> bool,
) -> Result<bool, AppError> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let canceled = if src_path.is_dir() {
            copy_dir_recursive_with_progress(&src_path, &dest_path, on_progress)?
        } else {
            copy_file_with_progress(&src_path, &dest_path, on_progress)?
        };
        if canceled {
            return Ok(true);
        }
    }
    Ok(false)
}

/// コピー（進捗報告つき）。呼び出し側で同名衝突が既に無いことを確認してから使う想定
/// （衝突時は常に上書き）。
pub fn copy_files_with_progress(
    sources: &[PathBuf],
    dest_dir: &Path,
    on_progress: &dyn Fn(u64, &str) -> bool,
) -> Result<usize, AppError> {
    let mut copied = 0;
    for source in sources {
        let file_name = source.file_name().ok_or(AppError::InvalidPath)?;
        let dest = dest_dir.join(file_name);
        let canceled = if source.is_dir() {
            copy_dir_recursive_with_progress(source, &dest, on_progress)?
        } else {
            copy_file_with_progress(source, &dest, on_progress)?
        };
        copied += 1;
        if canceled {
            return Ok(copied);
        }
    }
    Ok(copied)
}

/// 移動（進捗報告つき）。同一 FS 内なら rename、跨ぐ場合は copy + delete にフォールバック。
/// 呼び出し側で同名衝突が既に無いことを確認してから使う想定（衝突時は常に上書き）。
pub fn move_files_with_progress(
    sources: &[PathBuf],
    dest_dir: &Path,
    on_progress: &dyn Fn(u64, &str) -> bool,
) -> Result<usize, AppError> {
    let mut moved = 0;
    for source in sources {
        let file_name = source.file_name().ok_or(AppError::InvalidPath)?;
        let name_str = file_name.to_string_lossy().into_owned();
        let dest = dest_dir.join(file_name);

        if dest.exists() {
            if dest.is_dir() {
                fs::remove_dir_all(&dest)?;
            } else {
                fs::remove_file(&dest)?;
            }
        }

        let canceled = if fs::rename(source, &dest).is_ok() {
            !on_progress(path_size(&dest), &name_str)
        } else if source.is_dir() {
            let canceled = copy_dir_recursive_with_progress(source, &dest, on_progress)?;
            if !canceled {
                fs::remove_dir_all(source)?;
            }
            canceled
        } else {
            let canceled = copy_file_with_progress(source, &dest, on_progress)?;
            if !canceled {
                fs::remove_file(source)?;
            }
            canceled
        };

        moved += 1;
        if canceled {
            return Ok(moved);
        }
    }
    Ok(moved)
}

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// ディレクトリを再帰的にコピー
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use tempfile::TempDir;

    #[test]
    fn test_create_dir() {
        let parent = TempDir::new().unwrap();
        let result = create_dir(parent.path(), "new_dir");
        assert!(result.is_ok());
        assert!(parent.path().join("new_dir").is_dir());
    }

    #[test]
    fn test_create_dir_already_exists() {
        let parent = TempDir::new().unwrap();
        fs::create_dir(parent.path().join("existing")).unwrap();
        let result = create_dir(parent.path(), "existing");
        assert!(matches!(result, Err(AppError::FileAlreadyExists(_))));
    }

    #[test]
    fn test_create_file() {
        let parent = TempDir::new().unwrap();
        let result = create_file(parent.path(), "test.txt");
        assert!(result.is_ok());
        assert!(parent.path().join("test.txt").exists());
    }

    #[test]
    fn test_rename_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("old.txt");
        fs::write(&file, "content").unwrap();

        let result = rename(&file, "new.txt");
        assert!(result.is_ok());
        assert!(!dir.path().join("old.txt").exists());
        assert!(dir.path().join("new.txt").exists());
    }

    #[test]
    fn test_copy_files() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let src_file = src_dir.path().join("test.txt");
        fs::write(&src_file, "hello").unwrap();

        let result = copy_files(
            &[src_file.clone()],
            dest_dir.path(),
            &|_| ConflictResolution::Overwrite,
        );
        assert_eq!(result.unwrap(), 1);
        assert!(dest_dir.path().join("test.txt").exists());
        assert!(src_file.exists()); // 元ファイルは残る
    }

    #[test]
    fn test_move_files() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let src_file = src_dir.path().join("test.txt");
        fs::write(&src_file, "hello").unwrap();

        let result = move_files(
            &[src_file.clone()],
            dest_dir.path(),
            &|_| ConflictResolution::Overwrite,
        );
        assert_eq!(result.unwrap(), 1);
        assert!(dest_dir.path().join("test.txt").exists());
        assert!(!src_file.exists()); // 元ファイルは削除
    }

    #[test]
    fn test_total_size() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "12345").unwrap(); // 5 bytes
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("b.txt"), "1234567890").unwrap(); // 10 bytes

        assert_eq!(total_size(&[dir.path().to_path_buf()]), 15);
    }

    #[test]
    fn test_copy_files_with_progress_reports_full_size() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let src_file = src_dir.path().join("test.txt");
        fs::write(&src_file, "hello world").unwrap(); // 11 bytes

        let reported = std::sync::atomic::AtomicU64::new(0);
        let result = copy_files_with_progress(&[src_file.clone()], dest_dir.path(), &|delta, name| {
            assert_eq!(name, "test.txt");
            reported.fetch_add(delta, Ordering::Relaxed);
            true
        });

        assert_eq!(result.unwrap(), 1);
        assert_eq!(reported.load(Ordering::Relaxed), 11);
        assert!(dest_dir.path().join("test.txt").exists());
        assert!(src_file.exists());
    }

    #[test]
    fn test_copy_files_with_progress_cancel_stops_early() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let a = src_dir.path().join("a.txt");
        let b = src_dir.path().join("b.txt");
        fs::write(&a, "aaa").unwrap();
        fs::write(&b, "bbb").unwrap();

        // 最初のファイルの途中でキャンセルする
        let result = copy_files_with_progress(&[a, b], dest_dir.path(), &|_, _| false);

        assert_eq!(result.unwrap(), 1); // b.txt までは到達しない
        assert!(!dest_dir.path().join("b.txt").exists());
    }

    #[test]
    fn test_move_files_with_progress() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let src_file = src_dir.path().join("test.txt");
        fs::write(&src_file, "hello").unwrap();

        let result = move_files_with_progress(&[src_file.clone()], dest_dir.path(), &|_, _| true);

        assert_eq!(result.unwrap(), 1);
        assert!(dest_dir.path().join("test.txt").exists());
        assert!(!src_file.exists());
    }
}
