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
        //
        // ターミナルが極端に狭い（幅 < MIN_PANE_WIDTH*2）場合や、GUI 起動直後で
        // サイズが未確定（w=0）の場合、単純な clamp(MIN, w-MIN) では min > max となり
        // パニックする。max 側に合わせて min を縮め、常に min <= max を満たすようにする。
        let border_x = ((w as f32) * pane_ratio) as u16;
        let max_border = w.saturating_sub(MIN_PANE_WIDTH);
        let min_border = MIN_PANE_WIDTH.min(max_border);
        let border_x = border_x.clamp(min_border, max_border);

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

#[cfg(test)]
mod tests {
    use super::*;

    /// 幅 0・極小サイズでもパニックせずレイアウトを計算できること。
    /// （GUI ホスト起動直後に PTY サイズが届く前の 0 サイズ描画対策の回帰防止）
    #[test]
    fn calculate_does_not_panic_on_tiny_sizes() {
        for &(w, h) in &[(0u16, 0u16), (1, 1), (10, 3), (30, 5), (39, 10), (40, 10)] {
            let area = Rect::new(0, 0, w, h);
            let layout = AppLayout::calculate(area, 0.5);
            // 領域は元のターミナル幅を超えない
            assert!(layout.left_pane.width <= w);
            assert!(layout.right_pane.x.saturating_add(layout.right_pane.width) <= w);
        }
    }

    /// 通常サイズでは左右ペインが最小幅以上を確保すること。
    #[test]
    fn calculate_keeps_min_pane_width_on_normal_size() {
        let area = Rect::new(0, 0, 120, 40);
        let layout = AppLayout::calculate(area, 0.5);
        assert!(layout.left_pane.width >= MIN_PANE_WIDTH);
        assert!(layout.right_pane.width >= MIN_PANE_WIDTH);
    }
}
