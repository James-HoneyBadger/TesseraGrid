//! Export card: image, GIF, RLE and state file actions.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_export_section(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        let button_h = card.button_h;
        card.frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Export & State")
                .default_open(false)
                .show(ui, |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Export PNG..."),
                        )
                        .clicked()
                    {
                        self.export_png_dialog();
                    }

                    ui.separator();

                    // Save / load grid state
                    ui.horizontal(|ui| {
                        let w = (ui.available_width() - 8.0) / 2.0;
                        if ui
                            .add_sized([w, button_h], egui::Button::new("Save State..."))
                            .on_hover_text("Save current grid to a .tessera file")
                            .clicked()
                        {
                            self.save_state();
                        }
                        if ui
                            .add_sized([w, button_h], egui::Button::new("Load State..."))
                            .on_hover_text("Load a previously saved .tessera (or .lifegrid) file")
                            .clicked()
                        {
                            self.load_state();
                        }
                    });

                    ui.separator();

                    ui.checkbox(&mut self.sim.record_frames, "Record frames");
                    if self.sim.record_frames {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} frames buffered ({:.1} MB)",
                                self.sim.frame_count(),
                                self.sim.frame_bytes() as f64 / (1024.0 * 1024.0)
                            ))
                            .small(),
                        );
                    }
                    if ui
                        .add_enabled(
                            self.sim.frame_count() > 0,
                            egui::Button::new("Export GIF..."),
                        )
                        .clicked()
                    {
                        self.export_gif_dialog();
                    }
                    if ui
                        .add_enabled(
                            self.sim.frame_count() > 0,
                            egui::Button::new("Clear frames"),
                        )
                        .clicked()
                    {
                        self.sim.clear_frames();
                        self.set_status("Frame buffer cleared");
                    }
                });
        });
    }
}
