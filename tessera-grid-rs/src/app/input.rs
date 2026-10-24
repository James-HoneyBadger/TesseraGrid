use super::*;

impl TesseraGridApp {
    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Skip shortcuts when a text field is focused (e.g., custom rule input).
        // Never steal keys from a focused widget (text edit or focused button).
        if ctx.memory(|memory| memory.focused().is_some()) {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.running = !self.running;
                if self.running {
                    self.last_step = Instant::now();
                }
            }
            if i.key_pressed(egui::Key::S) {
                self.push_undo();
                self.do_step();
            }
            if i.key_pressed(egui::Key::N) {
                self.randomize();
            }
            if i.key_pressed(egui::Key::T) {
                self.theme = self.theme.next();
            }
            if i.key_pressed(egui::Key::R) {
                self.do_reset();
                let pat = self.selected_pattern.clone();
                self.sim.load_pattern(&pat);
            }
            if i.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && self.sim.undo() {
                self.running = false;
            }
            if i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
                && self.sim.redo()
            {
                self.running = false;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_state();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.load_state();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                self.export_png_dialog();
            }
            // Zoom
            let plus = i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals);
            let minus = i.key_pressed(egui::Key::Minus);
            if plus {
                self.cell_size = (self.cell_size + 1.0).min(64.0);
            }
            if minus {
                self.cell_size = (self.cell_size - 1.0).max(1.0);
            }

            // Zoom to fit
            if i.key_pressed(egui::Key::F) || (i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
                self.pending_fit = true;
            }

            // Arrow-key panning
            let pan = (self.cell_size * 5.0).max(10.0);
            if i.key_pressed(egui::Key::ArrowLeft) {
                self.viewport_offset.x -= pan;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                self.viewport_offset.x += pan;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                self.viewport_offset.y -= pan;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                self.viewport_offset.y += pan;
            }

            // Help
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::F1) {
                self.show_help = !self.show_help;
            }

            // Toggle 3-D view
            if i.key_pressed(egui::Key::V) {
                self.view_mode = match self.view_mode {
                    ViewMode::Flat => ViewMode::Isometric,
                    ViewMode::Isometric => ViewMode::Flat,
                };
                self.pending_fit = true;
            }

            // Selection shortcuts
            if i.key_pressed(egui::Key::Escape) {
                self.selection = None;
                self.selection_anchor = None;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                self.copy_selection();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X) {
                self.cut_selection();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                self.paste_clipboard();
            }
            if i.key_pressed(egui::Key::Delete) {
                self.clear_selection_cells();
            }
        });
    }
}
