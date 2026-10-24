//! Quick-action toolbar: run controls, speed, tools and view toggles.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let accent = self.theme.accent(&self.selected_mode);

            ui.label(
                egui::RichText::new("Tessera Grid")
                    .strong()
                    .size(16.0)
                    .color(accent),
            );
            ui.separator();

            // Play / Pause
            let play_label = if self.running { "Pause" } else { "Play" };
            if ui
                .button(play_label)
                .on_hover_text(if self.running {
                    "Pause (Space)"
                } else {
                    "Play (Space)"
                })
                .clicked()
            {
                self.running = !self.running;
                if self.running {
                    self.last_step = Instant::now();
                }
            }
            // Step
            if ui.button("Step").on_hover_text("Step (S)").clicked() {
                self.push_undo();
                self.do_step();
            }
            // Reset
            if ui.button("Reset").on_hover_text("Reset (R)").clicked() {
                self.do_reset();
                let pat = self.selected_pattern.clone();
                self.sim.load_pattern(&pat);
            }
            if ui
                .button("Randomize")
                .on_hover_text("Fill with a reproducible random soup (N)")
                .clicked()
            {
                self.randomize();
            }

            ui.separator();

            // Undo / Redo
            let can_undo = self.sim.can_undo();
            let can_redo = self.sim.can_redo();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .on_hover_text("Undo (Ctrl+Z)")
                .clicked()
                && self.sim.undo()
            {
                self.running = false;
            }
            if ui
                .add_enabled(can_redo, egui::Button::new("Redo"))
                .on_hover_text("Redo (Ctrl+Y)")
                .clicked()
                && self.sim.redo()
            {
                self.running = false;
            }

            ui.separator();

            // Run N steps
            if ui
                .button("Run N…")
                .on_hover_text("Run a fixed number of steps")
                .clicked()
            {
                self.show_run_n_dialog = true;
            }

            ui.separator();

            // Zoom to fit
            if ui
                .button("Fit")
                .on_hover_text("Zoom to fit grid (F)")
                .clicked()
            {
                self.pending_fit = true;
            }

            ui.separator();

            // Tool mode
            let paint_active = self.tool_mode == ToolMode::Paint;
            let sel_active = self.tool_mode == ToolMode::Select;
            if ui
                .selectable_label(paint_active, "Paint")
                .on_hover_text("Paint tool — draw cells (default)")
                .clicked()
            {
                self.tool_mode = ToolMode::Paint;
                self.selection = None;
                self.selection_anchor = None;
            }
            if ui
                .selectable_label(sel_active, "Select")
                .on_hover_text("Select tool — drag to select a region (copy/paste/clear)")
                .clicked()
            {
                self.tool_mode = ToolMode::Select;
            }

            ui.separator();

            // 3-D / isometric view toggle
            let iso_active = self.view_mode == ViewMode::Isometric;
            if ui
                .selectable_label(iso_active, "Iso 3D")
                .on_hover_text("Toggle isometric 3-D view (V)")
                .clicked()
            {
                self.view_mode = if iso_active {
                    ViewMode::Flat
                } else {
                    ViewMode::Isometric
                };
                self.pending_fit = true;
            }

            ui.separator();

            // Help
            if ui
                .button("Help")
                .on_hover_text("Keyboard shortcuts (H / F1)")
                .clicked()
            {
                self.show_help = !self.show_help;
            }

            // Version — right-aligned
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
            });
        });
    }
}
