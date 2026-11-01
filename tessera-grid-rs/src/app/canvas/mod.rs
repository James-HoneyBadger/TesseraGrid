//! Flat and isometric canvas rendering, viewport fitting, painting and selection.

mod flat;
mod iso;
mod paint;
mod viewport;

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_canvas(&mut self, ui: &mut egui::Ui) {
        // The grid buffers are borrowed from the simulation for the whole
        // frame; they are handed back when the canvas is done so a steady
        // frame neither allocates nor copies the grid.
        let frame = self.sim.take_render_frame();
        if self.view_mode == ViewMode::Isometric {
            self.ui_canvas_iso(ui, frame);
        } else {
            self.ui_canvas_flat(ui, frame);
        }
    }
}
