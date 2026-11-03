//! Grid resize dialog.

use crate::app::*;

/// Parse one grid side, rejecting anything the engine cannot honour instead of
/// silently substituting a default.
fn parse_side(label: &str, input: &str) -> Result<usize, String> {
    let value: usize = input
        .trim()
        .parse()
        .map_err(|_| format!("{label}: '{input}' is not a whole number"))?;
    if !(crate::sim::MIN_GRID_SIDE..=crate::sim::MAX_GRID_SIDE).contains(&value) {
        return Err(format!(
            "{label}: {value} is outside {}–{}",
            crate::sim::MIN_GRID_SIDE,
            crate::sim::MAX_GRID_SIDE
        ));
    }
    Ok(value)
}

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
                    egui::RichText::new(format!(
                        "Width and height must be {}–{} cells.",
                        crate::sim::MIN_GRID_SIDE,
                        crate::sim::MAX_GRID_SIDE
                    ))
                    .small()
                    .weak(),
                );
                if let Some(err) = &self.resize_error {
                    ui.colored_label(self.theme.colors().ok, err);
                }
                ui.horizontal(|ui| {
                    if ui.button("Resize").clicked() {
                        match (
                            parse_side("Width", &self.resize_w_input),
                            parse_side("Height", &self.resize_h_input),
                        ) {
                            (Ok(w), Ok(h)) => {
                                let old_ratio =
                                    self.sim.height() as f32 / self.sim.width().max(1) as f32;
                                let h = if self.resize_aspect {
                                    ((w as f32 * old_ratio).round() as usize)
                                        .clamp(crate::sim::MIN_GRID_SIDE, crate::sim::MAX_GRID_SIDE)
                                } else {
                                    h
                                };
                                let preserve = self.resize_preserve;
                                self.sim.resize(
                                    w,
                                    h,
                                    RuleSpec::new(
                                        self.selected_boundary,
                                        &self.custom_birth,
                                        &self.custom_survival,
                                    )
                                    .with_elementary(self.elementary_rule),
                                    preserve,
                                );
                                if !preserve {
                                    self.sim.load_pattern(&self.selected_pattern);
                                }
                                self.resize_error = None;
                                self.running = false;
                                self.viewport_offset = Vec2::ZERO;
                                self.pending_fit = true;
                                self.set_status(format!("Grid resized to {}×{}", w, h));
                                self.show_resize_dialog = false;
                            }
                            (Err(err), _) | (_, Err(err)) => self.resize_error = Some(err),
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.resize_error = None;
                        self.show_resize_dialog = false;
                    }
                });
            });
        if !open {
            self.show_resize_dialog = false;
        }
    }
}
