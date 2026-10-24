//! Run-for-N-generations dialog.

use crate::app::*;

impl TesseraGridApp {
    pub(crate) fn ui_run_n_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_run_n_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("Run N Steps")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                if self.run_n_pending > 0 {
                    let progress = 1.0 - self.run_n_pending as f32 / self.run_n_total.max(1) as f32;
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(240.0),
                    );
                    ui.label(format!(
                        "{} of {} steps remaining",
                        self.run_n_pending, self.run_n_total
                    ));
                    if ui.button("Stop").clicked() {
                        self.run_n_pending = 0;
                        self.set_status("Run cancelled");
                        self.show_run_n_dialog = false;
                    }
                } else {
                    ui.label("Number of steps:");
                    ui.text_edit_singleline(&mut self.run_n_input);
                    ui.horizontal(|ui| {
                        if ui.button("Run").clicked() {
                            if let Ok(n) = self.run_n_input.trim().parse::<u64>() {
                                if n > 0 {
                                    self.push_undo();
                                    self.run_n_total = n.min(1_000_000);
                                    self.run_n_pending = self.run_n_total;
                                    self.running = false;
                                }
                            } else {
                                self.set_status("Enter a positive number of steps");
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_run_n_dialog = false;
                        }
                    });
                }
            });
        if !open {
            self.show_run_n_dialog = false;
        }
    }
}
