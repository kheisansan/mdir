// ui/layout.rs - 画面レイアウト計算
//
// ターミナルサイズとペイン比率から各領域の Rect を計算する。
// 全ウィジェット描画の基盤となる共通レイアウト情報を提供する。

use ratatui::layout::Rect;

/// 最小ターミナルサイズ
#[allow(dead_code)]
pub const MIN_WIDTH: u16 = 60;
#[allow(dead_code)]
pub const MIN_HEIGHT: u16 = 10;
/// ペインの最小幅
pub const MIN_PANE_WIDTH: u16 = 20;

/// 画面レイアウト（各領域の Rect）
pub struct AppLayout {
    pub header: Rect,
    pub left_path: Rect,
    pub right_path: Rect,
    pub left_pane: Rect,
    pub right_pane: Rect,
    pub statusbar: Rect,
    pub bottom_bar: Rect,
}

impl AppLayout {
    /// ターミナルサイズとペイン比率からレイアウトを計算
    pub fn calculate(area: Rect, pane_ratio: f32) -> Self {
        let w = area.width;
        let h = area.height;

        // ペイン境界の x 座標（最小幅を保証）
        let border_x = ((w as f32) * pane_ratio) as u16;
        let border_x = border_x.clamp(MIN_PANE_WIDTH, w.saturating_sub(MIN_PANE_WIDTH));

        let right_width = w.saturating_sub(border_x);
        let pane_height = h.saturating_sub(4); // header(1) + path(1) + status(1) + bottom(1)

        Self {
            header: Rect::new(area.x, area.y, w, 1),
            left_path: Rect::new(area.x, area.y + 1, border_x, 1),
            right_path: Rect::new(area.x + border_x, area.y + 1, right_width, 1),
            left_pane: Rect::new(area.x, area.y + 2, border_x, pane_height),
            right_pane: Rect::new(area.x + border_x, area.y + 2, right_width, pane_height),
            statusbar: Rect::new(area.x, h.saturating_sub(2), w, 1),
            bottom_bar: Rect::new(area.x, h.saturating_sub(1), w, 1),
        }
    }

    /// ファイルペインの表示可能行数
    pub fn pane_visible_rows(&self) -> usize {
        self.left_pane.height as usize
    }

    /// ペイン境界の x 座標
    #[allow(dead_code)]
    pub fn border_x(&self) -> u16 {
        self.left_pane.width
    }
}
