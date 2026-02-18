// command/history.rs - コマンド履歴管理
//
// コマンドモードで入力されたコマンドの履歴を保持し、
// ↑/↓ キーでの履歴参照とファイルへの永続化を提供する。

use std::path::PathBuf;

const MAX_HISTORY_SIZE: usize = 500;
const HISTORY_FILE: &str = "history";

/// コマンド履歴
pub struct CommandHistory {
    /// 履歴エントリ（インデックス 0 が最新）
    entries: Vec<String>,
    /// 履歴ファイルパス
    file_path: PathBuf,
}

impl CommandHistory {
    /// 設定ディレクトリから履歴を読み込んで初期化
    pub fn load(config_dir: &std::path::Path) -> Self {
        let file_path = config_dir.join(HISTORY_FILE);
        let entries = std::fs::read_to_string(&file_path)
            .unwrap_or_default()
            .lines()
            .rev() // ファイルは古い順→読み込み後は新しい順
            .take(MAX_HISTORY_SIZE)
            .map(|s| s.to_string())
            .collect();
        Self { entries, file_path }
    }

    /// 空の履歴を作成（テスト用）
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            file_path: PathBuf::new(),
        }
    }

    /// コマンドを履歴に追加（直前と同一なら無視）
    pub fn push(&mut self, command: &str) {
        let cmd = command.trim().to_string();
        if cmd.is_empty() {
            return;
        }
        // 重複除去（直前と同じコマンド）
        if self.entries.first().map(|s| s.as_str()) == Some(cmd.as_str()) {
            return;
        }
        self.entries.insert(0, cmd);
        if self.entries.len() > MAX_HISTORY_SIZE {
            self.entries.truncate(MAX_HISTORY_SIZE);
        }
    }

    /// インデックスで履歴を取得（0 = 最新）
    pub fn get(&self, index: usize) -> Option<&str> {
        self.entries.get(index).map(|s| s.as_str())
    }

    /// 履歴件数
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 履歴をファイルに保存
    pub fn save(&self) -> Result<(), std::io::Error> {
        // ディレクトリが無ければ作成
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content: String = self
            .entries
            .iter()
            .rev() // ファイルには古い順で保存
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&self.file_path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_get() {
        let mut h = CommandHistory::empty();
        h.push("help");
        h.push("cd /tmp");
        assert_eq!(h.get(0), Some("cd /tmp"));
        assert_eq!(h.get(1), Some("help"));
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_skip_duplicate() {
        let mut h = CommandHistory::empty();
        h.push("help");
        h.push("help");
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn test_skip_empty() {
        let mut h = CommandHistory::empty();
        h.push("");
        h.push("   ");
        assert_eq!(h.len(), 0);
    }
}
