//! Grid resize dialog.

use crate::app::*;

impl TesseraGridApp {
    pub(crate) fn ui_resize_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_resize_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("Resize Grid")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.text_edit_singleline(&mut self.resize_w_input);
                });
                ui.horizontal(|ui| {
                    ui.label("Height:");
                    ui.text_edit_singleline(&mut self.resize_h_input);
                });
                ui.checkbox(
                    &mut self.resize_preserve,
                    "Preserve current pattern (copy overlapping cells)",
                );
                ui.checkbox(&mut self.resize_aspect, "Keep aspect ratio");
                ui.label(
                    egui::RichText::new("New grids are clamped to 10–1000 cells.")
                        .small()
                        .weak(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Resize").clicked() {
                        let w = self
                            .resize_w_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        let old_ratio = self.sim.height() as f32 / self.sim.width().max(1) as f32;
                        let mut h = self
                            .resize_h_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        if self.resize_aspect {
                            h = ((w as f32 * old_ratio).round() as usize).clamp(10, 1000);
                        }
                        let preserve = self.resize_preserve;
                        self.sim.resize(
                            w,
                            h,
                            RuleSpec::new(
                                self.selected_boundary,
                                &self.custom_birth,
                                &self.custom_survival,
                            ),
                            preserve,
                        );
                        if !preserve {
                            self.sim.load_pattern(&self.selected_pattern);
                        }
                        self.running = false;
                        self.viewport_offset = Vec2::ZERO;
                        self.pending_fit = true;
                        self.set_status(format!("Grid resized to {}×{}", w, h));
                        self.show_resize_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_resize_dialog = false;
                    }
                });
            });
        if !open {
            self.show_resize_dialog = false;
        }
    }
}
