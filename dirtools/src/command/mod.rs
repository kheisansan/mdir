// command/mod.rs - コマンドモジュール
//
// vi スタイルの ':' コマンドモードを構成するサブモジュール群を統合する。
// パース → 実行 → 履歴保存 → オートコンプリートの一連の流れを提供する。

pub mod completer;
pub mod executor;
pub mod history;
pub mod parser;

use history::CommandHistory;

/// コマンドモードの入力状態
pub struct CommandState {
    /// 現在の入力文字列
    pub input: String,
    /// カーソル位置（バイト単位）
    pub cursor_pos: usize,
    /// コマンド履歴
    pub history: CommandHistory,
    /// 履歴参照中のインデックス（None = 参照していない）
    pub history_index: Option<usize>,
    /// オートコンプリート候補
    pub completions: Vec<String>,
    /// 選択中の補完候補インデックス
    pub completion_index: Option<usize>,
    /// コマンド実行前の入力値（履歴参照時に元に戻すため）
    pub saved_input: Option<String>,
}

impl CommandState {
    /// 新規作成
    pub fn new(history: CommandHistory) -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            history,
            history_index: None,
            completions: Vec::new(),
            completion_index: None,
            saved_input: None,
        }
    }

    /// コマンドモード開始時のリセット
    pub fn reset(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        self.completions.clear();
        self.completion_index = None;
        self.saved_input = None;
    }

    /// 文字を入力位置に挿入
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.clear_completions();
    }

    /// Backspace: カーソル前の文字を削除
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            // 前の文字のバイト位置を探す
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
            self.clear_completions();
        }
    }

    /// 履歴の前のコマンドを表示
    pub fn history_prev(&mut self) {
        let next_index = match self.history_index {
            None => {
                // 現在の入力を保存
                self.saved_input = Some(self.input.clone());
                0
            }
            Some(i) => i + 1,
        };

        if let Some(cmd) = self.history.get(next_index) {
            self.input = cmd.to_string();
            self.cursor_pos = self.input.len();
            self.history_index = Some(next_index);
        }
    }

    /// 履歴の次のコマンドを表示
    pub fn history_next(&mut self) {
        match self.history_index {
            None => {} // 既に最新
            Some(0) => {
                // 保存していた入力に戻る
                if let Some(saved) = self.saved_input.take() {
                    self.input = saved;
                } else {
                    self.input.clear();
                }
                self.cursor_pos = self.input.len();
                self.history_index = None;
            }
            Some(i) => {
                if let Some(cmd) = self.history.get(i - 1) {
                    self.input = cmd.to_string();
                    self.cursor_pos = self.input.len();
                    self.history_index = Some(i - 1);
                }
            }
        }
    }

    /// 補完候補をクリア
    fn clear_completions(&mut self) {
        self.completions.clear();
        self.completion_index = None;
    }
}
