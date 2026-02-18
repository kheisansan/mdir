// fs/mod.rs - ファイルシステムモジュール
//
// ファイルエントリモデル、ペイン状態、ファイル操作を統合する。
// 左右ペインそれぞれが PaneState を持ち、ディレクトリ内容の読み込み・
// ソート・フィルタ・マーキングの状態を管理する。

pub mod entry;
pub mod operations;

use crate::error::AppError;
use entry::{FileEntry, SortOrder};
use std::collections::HashSet;
use std::path::PathBuf;

/// 1つのファイルペインの状態
pub struct PaneState {
    /// カレントディレクトリの絶対パス
    pub current_dir: PathBuf,
    /// 表示用エントリ一覧（ソート・フィルタ適用済み）
    pub entries: Vec<FileEntry>,
    /// カーソル位置（0-indexed）
    pub cursor: usize,
    /// スクロールオフセット（表示先頭行のインデックス）
    pub scroll_offset: usize,
    /// マーク済みファイル名セット（インデックスではなく名前で管理）
    pub marked: HashSet<String>,
    /// ソート設定
    pub sort_order: SortOrder,
    /// 隠しファイル表示フラグ
    pub show_hidden: bool,
    /// Git ブランチ名キャッシュ（None = Git リポジトリではない）
    pub git_branch: Option<String>,
}

impl PaneState {
    /// 指定ディレクトリで PaneState を初期化
    pub fn new(path: PathBuf, show_hidden: bool) -> Result<Self, AppError> {
        let canonical = if path.exists() {
            path.canonicalize().unwrap_or(path)
        } else {
            path
        };

        let git_branch = crate::utils::git::current_branch(&canonical);
        let mut state = Self {
            current_dir: canonical,
            entries: Vec::new(),
            cursor: 0,
            scroll_offset: 0,
            marked: HashSet::new(),
            sort_order: SortOrder::default(),
            show_hidden,
            git_branch,
        };
        state.refresh()?;
        Ok(state)
    }

    /// ディレクトリ内容を再読み込みし、ソート・フィルタを適用
    ///
    /// 先頭に `.`（カレント）と `..`（親、ルートでは非表示）を挿入する。
    pub fn refresh(&mut self) -> Result<(), AppError> {
        self.git_branch = crate::utils::git::current_branch(&self.current_dir);
        let mut entries = Vec::new();

        for dir_entry in std::fs::read_dir(&self.current_dir)? {
            let dir_entry = dir_entry?;
            match FileEntry::from_dir_entry(&dir_entry) {
                Ok(entry) => {
                    // 隠しファイルフィルタ
                    if !self.show_hidden && entry.is_hidden {
                        continue;
                    }
                    entries.push(entry);
                }
                Err(_) => continue, // メタデータ取得失敗は無視
            }
        }

        // ソート適用
        self.sort_order.sort(&mut entries);

        // . と .. エントリを先頭に挿入
        let mut dot_entries = Vec::with_capacity(2 + entries.len());
        dot_entries.push(FileEntry::create_dot_entry(".", self.current_dir.clone()));
        // ルートディレクトリでない場合のみ .. を追加
        if let Some(parent) = self.current_dir.parent() {
            dot_entries.push(FileEntry::create_dot_entry("..", parent.to_path_buf()));
        }
        dot_entries.append(&mut entries);
        self.entries = dot_entries;

        // カーソル位置の補正（エントリ数が減った場合に範囲外にならないように）
        self.clamp_cursor();

        Ok(())
    }

    /// カーソルを delta だけ移動（範囲内にクランプ）
    pub fn move_cursor(&mut self, delta: i32) {
        let new_pos = self.cursor as i32 + delta;
        self.cursor = new_pos.clamp(0, self.max_cursor() as i32) as usize;
    }

    /// カーソル位置のエントリを取得
    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// 選択対象エントリを取得（マーク済みがあればそれ、なければカーソル位置）
    ///
    /// `.` と `..` は操作対象から除外する。
    pub fn selected_entries(&self) -> Vec<&FileEntry> {
        if self.marked.is_empty() {
            self.current_entry()
                .filter(|e| e.name != "." && e.name != "..")
                .into_iter()
                .collect()
        } else {
            self.entries
                .iter()
                .filter(|e| e.name != "." && e.name != ".." && self.marked.contains(&e.name))
                .collect()
        }
    }

    /// 選択対象のパス一覧を取得
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selected_entries()
            .iter()
            .map(|e| e.path.clone())
            .collect()
    }

    /// カーソル位置のファイルのマーク状態をトグル
    ///
    /// `.` と `..` はマーク対象外。
    pub fn toggle_mark(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor) {
            // . と .. はマーク不可
            if entry.name == "." || entry.name == ".." {
                return;
            }
            let name = entry.name.clone();
            if self.marked.contains(&name) {
                self.marked.remove(&name);
            } else {
                self.marked.insert(name);
            }
        }
    }

    /// 全選択/全解除をトグル
    ///
    /// `.` と `..` は対象外。
    pub fn toggle_mark_all(&mut self) {
        let real_names: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.name != "." && e.name != "..")
            .map(|e| e.name.clone())
            .collect();
        if self.marked.len() == real_names.len() {
            self.marked.clear();
        } else {
            self.marked = real_names.into_iter().collect();
        }
    }

    /// マークをクリア
    pub fn clear_marks(&mut self) {
        self.marked.clear();
    }

    /// 指定ディレクトリへ移動
    ///
    /// 移動後、カーソルは `..` があればそこに、なければ最初の実エントリに配置する。
    pub fn navigate_to(&mut self, path: PathBuf) -> Result<(), AppError> {
        let canonical = path.canonicalize().map_err(|_| AppError::PathNotFound(path.clone()))?;
        if !canonical.is_dir() {
            return Err(AppError::NotADirectory(canonical));
        }
        self.current_dir = canonical;
        self.cursor = 0;
        self.scroll_offset = 0;
        self.clear_marks();
        self.refresh()?;

        // カーソルを ".." に配置（存在しない場合は最初の実エントリ）
        if let Some(idx) = self.entries.iter().position(|e| e.name == "..") {
            self.cursor = idx;
        } else if let Some(idx) = self.entries.iter().position(|e| e.name != ".") {
            self.cursor = idx;
        }
        Ok(())
    }

    /// 親ディレクトリへ移動（カーソルを元いたディレクトリに合わせる）
    pub fn navigate_parent(&mut self) -> Result<(), AppError> {
        let current_name = self
            .current_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        if let Some(parent) = self.current_dir.parent().map(|p| p.to_path_buf()) {
            self.navigate_to(parent)?;

            // 元いたディレクトリにカーソルを合わせる
            if let Some(name) = current_name {
                if let Some(idx) = self.entries.iter().position(|e| e.name == name) {
                    self.cursor = idx;
                }
            }
        }
        Ok(())
    }

    /// カーソルを先頭に移動
    pub fn jump_top(&mut self) {
        self.cursor = 0;
    }

    /// カーソルを末尾に移動
    pub fn jump_bottom(&mut self) {
        self.cursor = self.max_cursor();
    }

    /// スクロール位置をカーソルに追従させる
    pub fn adjust_scroll(&mut self, visible_height: usize) {
        let margin = 3usize;
        if visible_height == 0 {
            return;
        }

        // カーソルが上にはみ出す場合
        if self.cursor < self.scroll_offset + margin {
            self.scroll_offset = self.cursor.saturating_sub(margin);
        }
        // カーソルが下にはみ出す場合
        if self.cursor >= self.scroll_offset + visible_height - margin {
            self.scroll_offset = self
                .cursor
                .saturating_sub(visible_height - margin - 1);
        }
    }

    /// マーク済みファイル数のサマリー文字列
    pub fn mark_summary(&self) -> Option<String> {
        if self.marked.is_empty() {
            return None;
        }
        let total_size: u64 = self
            .entries
            .iter()
            .filter(|e| self.marked.contains(&e.name))
            .map(|e| e.size)
            .sum();
        Some(format!(
            "{}件選択中（{}）",
            self.marked.len(),
            crate::utils::format::format_size(total_size),
        ))
    }

    // --- 内部ヘルパー ---

    /// カーソルの最大値
    fn max_cursor(&self) -> usize {
        self.entries.len().saturating_sub(1)
    }

    /// カーソルを有効範囲内にクランプ
    fn clamp_cursor(&mut self) {
        if !self.entries.is_empty() {
            self.cursor = self.cursor.min(self.max_cursor());
        } else {
            self.cursor = 0;
        }
    }
}
