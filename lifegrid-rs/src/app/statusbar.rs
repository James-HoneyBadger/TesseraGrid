use super::*;

impl LifeGridApp {
    pub(super) fn ui_statusbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (gw, gh) = (self.sim.width(), self.sim.height());
            let pop = self.sim.population();
            let density = self.sim.density() * 100.0;

            ui.label(egui::RichText::new(format!("Gen {}", self.sim.generation)).strong());
            ui.separator();
            ui.label(format!("Pop {pop}"));
            ui.separator();
            ui.label(format!("Density {density:.1}%"));
            ui.separator();
            ui.label(format!("{gw}×{gh}"));
            ui.separator();
            ui.label(format!("{:.0} gen/s", self.gen_rate));
            ui.separator();
            ui.colored_label(mode_accent_color(&self.selected_mode), &self.selected_mode);
            if let Some((x, y, state)) = self.hover_cell {
                ui.separator();
                ui.label(format!("Cell ({x}, {y}) · state {state}"));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0} fps", self.fps))
                        .small()
                        .weak(),
                );
                if let Some((msg, ts)) = &self.status_msg {
                    if ts.elapsed() < Duration::from_secs(4) {
                        ui.separator();
                        ui.colored_label(Color32::from_rgb(100, 220, 100), msg);
                    }
                }
            });
            if self
                .status_msg
                .as_ref()
                .is_some_and(|(_, ts)| ts.elapsed() >= Duration::from_secs(4))
            {
                self.status_msg = None;
            }
        });
    }
}
