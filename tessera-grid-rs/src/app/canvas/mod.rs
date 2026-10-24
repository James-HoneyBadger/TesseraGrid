//! Flat and isometric canvas rendering, viewport fitting, painting and selection.

mod flat;
mod iso;
mod paint;
mod viewport;

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_canvas(&mut self, ui: &mut egui::Ui) {
        if self.view_mode == ViewMode::Isometric {
            self.ui_canvas_iso(ui);
        } else {
            self.ui_canvas_flat(ui);
        }
    }
}
