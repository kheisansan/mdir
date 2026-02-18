// fs/operations.rs - ファイル操作（コピー/移動/削除/リネーム/作成）
//
// 全てのファイル操作をこのモジュールに集約し、
// エラーハンドリングと進捗通知を統一的に行う。

use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};

/// ファイル名衝突時の解決方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConflictResolution {
    Overwrite,
    Skip,
    Cancel,
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
        if dest.exists() {
            match on_conflict(&dest) {
                ConflictResolution::Overwrite => {}
                ConflictResolution::Skip => continue,
                ConflictResolution::Cancel => return Ok(copied),
            }
        }

        if source.is_dir() {
            copy_dir_recursive(source, &dest)?;
        } else {
            fs::copy(source, &dest)?;
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

        // まず rename を試行（高速、同一FS内の場合）
        if fs::rename(source, &dest).is_err() {
            // 別 FS の場合: copy + delete
            if source.is_dir() {
                copy_dir_recursive(source, &dest)?;
                fs::remove_dir_all(source)?;
            } else {
                fs::copy(source, &dest)?;
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
}
