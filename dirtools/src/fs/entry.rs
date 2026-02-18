// fs/entry.rs - ファイル/ディレクトリエントリのデータモデル
//
// ファイルシステム上の1エントリ（ファイルまたはディレクトリ）を表す構造体と、
// ソート・フィルタに必要な型を定義する。
//
// クロスプラットフォーム対応:
// - Unix: パーミッションモード (mode) と実行可能ビットを使用
// - Windows: ファイル属性 (file_attributes) と隠しファイル属性を使用

use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::SystemTime;

/// ファイル/ディレクトリの種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryType {
    Directory,
    File,
    Executable,
    Symlink,
}

/// ファイルエントリ（1ファイル/ディレクトリの情報）
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub entry_type: EntryType,
    pub size: u64,
    pub modified: SystemTime,
    pub created: Option<SystemTime>,
    /// Unix パーミッションモードビット（Windows では 0）
    pub mode: u32,
    /// 隠しファイルフラグ
    /// Unix: 名前が '.' で始まる
    /// Windows: 名前が '.' で始まる OR FILE_ATTRIBUTE_HIDDEN 属性
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
    /// 拡張子（ソート用にキャッシュ）
    pub extension: Option<String>,
}

impl FileEntry {
    /// `std::fs::DirEntry` から `FileEntry` を構築
    ///
    /// メタデータ取得に失敗した場合はデフォルト値で埋める。
    pub fn from_dir_entry(entry: &std::fs::DirEntry) -> std::io::Result<Self> {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        // シンボリックリンクの場合は lstat 相当の情報を使う
        let symlink_meta = entry.metadata()?;
        let is_symlink = symlink_meta.is_symlink();

        // リンク先のメタデータ（リンク切れの場合は lstat を使う）
        let meta = if is_symlink {
            std::fs::metadata(&path).unwrap_or_else(|_| symlink_meta.clone())
        } else {
            symlink_meta.clone()
        };

        // --- プラットフォーム別のメタデータ取得 ---
        let (mode, is_hidden, entry_type) = Self::platform_metadata(&name, &meta, is_symlink);

        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase());

        let symlink_target = if is_symlink {
            std::fs::read_link(&path).ok()
        } else {
            None
        };

        Ok(Self {
            name,
            path,
            entry_type,
            size: meta.len(),
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            created: meta.created().ok(),
            mode,
            is_hidden,
            is_symlink,
            symlink_target,
            extension,
        })
    }

    /// Unix 向けメタデータ取得
    #[cfg(unix)]
    fn platform_metadata(
        name: &str,
        meta: &std::fs::Metadata,
        is_symlink: bool,
    ) -> (u32, bool, EntryType) {
        use std::os::unix::fs::PermissionsExt;

        let mode = meta.permissions().mode();
        let is_hidden = name.starts_with('.');
        let entry_type = if meta.is_dir() {
            EntryType::Directory
        } else if is_symlink {
            EntryType::Symlink
        } else if mode & 0o111 != 0 {
            EntryType::Executable
        } else {
            EntryType::File
        };

        (mode, is_hidden, entry_type)
    }

    /// Windows 向けメタデータ取得
    #[cfg(windows)]
    fn platform_metadata(
        name: &str,
        meta: &std::fs::Metadata,
        is_symlink: bool,
    ) -> (u32, bool, EntryType) {
        use std::os::windows::fs::MetadataExt;

        let attrs = meta.file_attributes();
        // Windows 隠しファイル属性 (FILE_ATTRIBUTE_HIDDEN = 0x2)
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        let is_hidden = name.starts_with('.') || (attrs & FILE_ATTRIBUTE_HIDDEN != 0);

        // Windows にはパーミッションモードがないため mode = 0
        // ただしファイル属性から読み取り専用を判定可能（将来用）
        let mode = 0u32;

        let entry_type = if meta.is_dir() {
            EntryType::Directory
        } else if is_symlink {
            EntryType::Symlink
        } else {
            // Windows には実行可能ビットがないため、拡張子で判定
            let is_exe = std::path::Path::new(name)
                .extension()
                .map(|ext| {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    matches!(ext_lower.as_str(), "exe" | "cmd" | "bat" | "com" | "ps1")
                })
                .unwrap_or(false);
            if is_exe {
                EntryType::Executable
            } else {
                EntryType::File
            }
        };

        (mode, is_hidden, entry_type)
    }

    /// パーミッションを表示用文字列に変換
    /// Unix: "-rwxr-xr-x" 形式
    /// Windows: 属性文字列（読み取り専用等）
    pub fn permission_string(&self) -> String {
        crate::utils::format::format_permissions(
            self.mode,
            self.entry_type == EntryType::Directory,
            self.is_symlink,
        )
    }

    /// ディレクトリかどうか
    pub fn is_dir(&self) -> bool {
        self.entry_type == EntryType::Directory
    }

    /// `.` (カレント) または `..` (親) ディレクトリの合成エントリを作成
    ///
    /// ファイルリスト先頭に表示するための特殊エントリ。
    /// メタデータは実際のディレクトリから取得する。
    pub fn create_dot_entry(name: &str, path: PathBuf) -> Self {
        let meta = std::fs::metadata(&path).ok();
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let created = meta.as_ref().and_then(|m| m.created().ok());

        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            meta.as_ref()
                .map(|m| m.permissions().mode())
                .unwrap_or(0)
        };
        #[cfg(windows)]
        let mode = 0u32;

        Self {
            name: name.to_string(),
            path,
            entry_type: EntryType::Directory,
            size: 0,
            modified,
            created,
            mode,
            is_hidden: false,
            is_symlink: false,
            symlink_target: None,
            extension: None,
        }
    }
}

// ---------------------------------------------------------------------------
// ソート関連
// ---------------------------------------------------------------------------

/// ソート基準
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortCriteria {
    Name,
    Size,
    Date,
    Extension,
}

/// ソート設定（基準 + 方向 + ディレクトリ優先）
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
    /// エントリ一覧をこのソート設定に従ってソートする
    pub fn sort(&self, entries: &mut [FileEntry]) {
        entries.sort_by(|a, b| {
            // 1. ディレクトリ優先（有効時）
            if self.dirs_first {
                let a_dir = a.is_dir();
                let b_dir = b.is_dir();
                if a_dir != b_dir {
                    return if a_dir { Ordering::Less } else { Ordering::Greater };
                }
            }

            // 2. 指定基準でソート
            let ord = match self.criteria {
                SortCriteria::Name => {
                    a.name.to_lowercase().cmp(&b.name.to_lowercase())
                }
                SortCriteria::Size => a.size.cmp(&b.size),
                SortCriteria::Date => a.modified.cmp(&b.modified),
                SortCriteria::Extension => {
                    let a_ext = a.extension.as_deref().unwrap_or("");
                    let b_ext = b.extension.as_deref().unwrap_or("");
                    a_ext.cmp(b_ext)
                }
            };

            // 3. 昇順/降順
            if self.ascending { ord } else { ord.reverse() }
        });
    }

    /// ソート基準の表示名
    #[allow(dead_code)]
    pub fn criteria_label(&self) -> &str {
        match self.criteria {
            SortCriteria::Name => "Name",
            SortCriteria::Size => "Size",
            SortCriteria::Date => "Date",
            SortCriteria::Extension => "Ext",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, entry_type: EntryType, size: u64) -> FileEntry {
        FileEntry {
            name: name.to_string(),
            path: PathBuf::from(name),
            entry_type,
            size,
            modified: SystemTime::UNIX_EPOCH,
            created: None,
            mode: 0o644,
            is_hidden: name.starts_with('.'),
            is_symlink: false,
            symlink_target: None,
            extension: PathBuf::from(name)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase()),
        }
    }

    #[test]
    fn test_sort_by_name() {
        let mut entries = vec![
            make_entry("charlie.txt", EntryType::File, 100),
            make_entry("alpha.txt", EntryType::File, 200),
            make_entry("bravo.txt", EntryType::File, 50),
        ];
        let order = SortOrder {
            criteria: SortCriteria::Name,
            ascending: true,
            dirs_first: false,
        };
        order.sort(&mut entries);
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha.txt", "bravo.txt", "charlie.txt"]);
    }

    #[test]
    fn test_sort_dirs_first() {
        let mut entries = vec![
            make_entry("file_b", EntryType::File, 0),
            make_entry("dir_a", EntryType::Directory, 0),
            make_entry("file_a", EntryType::File, 0),
        ];
        SortOrder::default().sort(&mut entries);
        assert_eq!(entries[0].name, "dir_a");
    }

    #[test]
    fn test_sort_by_size() {
        let mut entries = vec![
            make_entry("small", EntryType::File, 10),
            make_entry("big", EntryType::File, 1000),
            make_entry("medium", EntryType::File, 500),
        ];
        let order = SortOrder {
            criteria: SortCriteria::Size,
            ascending: true,
            dirs_first: false,
        };
        order.sort(&mut entries);
        assert_eq!(entries[0].name, "small");
        assert_eq!(entries[2].name, "big");
    }

    #[test]
    fn test_sort_descending() {
        let mut entries = vec![
            make_entry("a", EntryType::File, 0),
            make_entry("c", EntryType::File, 0),
            make_entry("b", EntryType::File, 0),
        ];
        let order = SortOrder {
            criteria: SortCriteria::Name,
            ascending: false,
            dirs_first: false,
        };
        order.sort(&mut entries);
        assert_eq!(entries[0].name, "c");
    }
}
