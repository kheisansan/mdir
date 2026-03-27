// mode.rs - アプリケーションモード管理
//
// vi エディタのモード概念に準拠した状態機械。
// Normal → Command → Help / Dialog の遷移を管理する。

/// アプリケーションモード（vi のモード概念に対応）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// 通常モード: ファイルブラウジング・操作
    Normal,
    /// コマンドモード: ':' プロンプトでコマンド入力
    Command,
    /// 検索モード: '/' プロンプトでファイル名検索入力
    Search,
    /// ヘルプモード: 全画面ヘルプ表示
    Help,
    /// ダイアログモード: モーダルダイアログ表示中
    Dialog,
}

impl AppMode {
    /// コマンドモードへの遷移が可能か（ノーマルモードからのみ）
    pub fn can_enter_command(&self) -> bool {
        matches!(self, Self::Normal)
    }

    /// ヘルプモードへの遷移が可能か
    #[allow(dead_code)]
    pub fn can_enter_help(&self) -> bool {
        matches!(self, Self::Normal | Self::Command)
    }

    /// ダイアログへの遷移が可能か
    #[allow(dead_code)]
    pub fn can_enter_dialog(&self) -> bool {
        matches!(self, Self::Normal)
    }
}
