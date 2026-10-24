//! Timeline scrubber: step back through recently simulated generations.

use super::*;

impl TesseraGridApp {
    /// Draw the scrubber bar beneath the canvas.
    pub(super) fn ui_timeline(&mut self, ui: &mut egui::Ui) {
        let len = self.sim.timeline_len();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Timeline").small().weak());

            if len < 2 {
                ui.weak("History collects while the simulation runs.");
                return;
            }

            let scrubbed = self.sim.timeline_pos();
            let live_index = len - 1;
            let mut index = scrubbed.unwrap_or(live_index);
            let slider = ui.add(
                egui::Slider::new(&mut index, 0..=live_index)
                    .show_value(false)
                    .trailing_fill(true),
            );
            if slider.changed() && self.sim.scrub(index) {
                self.running = false;
                self.set_status(format!("Scrubbed to generation {index}"));
            }

            let generation = self.sim.timeline_generation(index).unwrap_or_default();
            ui.label(egui::RichText::new(format!("Gen {generation}")).monospace());
            ui.weak(format!("{} steps buffered", len));

            if scrubbed.is_some() && ui.button("Return to live").clicked() {
                self.sim.resume_live();
                self.set_status("Back to the newest generation");
            }
        });
    }
}
