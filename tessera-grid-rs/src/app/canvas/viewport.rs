//! Viewport fitting for the flat and isometric canvases.

use super::*;

impl TesseraGridApp {
    pub(super) fn zoom_to_fit(&mut self) {
        let cw = self.canvas_rect.width();
        let ch = self.canvas_rect.height();
        if cw <= 0.0 || ch <= 0.0 {
            return;
        }
        let (gw, gh) = {
            let g = self.sim.grid();
            (g.width as f32, g.height as f32)
        };
        self.cell_size = (cw / gw).min(ch / gh).clamp(1.0, 64.0);
        let total_w = gw * self.cell_size;
        let total_h = gh * self.cell_size;
        self.viewport_offset = Vec2::new(
            ((total_w - cw) / 2.0).max(0.0),
            ((total_h - ch) / 2.0).max(0.0),
        );
    }

    pub(super) fn iso_fit(&mut self) {
        let cw = self.canvas_rect.width();
        let ch = self.canvas_rect.height();
        if cw <= 0.0 || ch <= 0.0 {
            return;
        }
        let (gw, gh) = {
            let g = self.sim.grid();
            (g.width as f32, g.height as f32)
        };
        // Isometric screen extent: width ≈ (gw+gh)*cs/2, height ≈ (gw+gh)*cs/4 + height_margin
        let h_margin = 4.0 * self.iso_height; // state-units × cs/2 × iso_h, simplified
        let cs_w = cw * 2.0 / (gw + gh).max(1.0);
        let cs_h = ch / ((gw + gh) / 4.0 + h_margin).max(1.0);
        self.cell_size = cs_w.min(cs_h).clamp(1.0, 64.0);
        self.iso_offset = Vec2::ZERO;
    }
}
