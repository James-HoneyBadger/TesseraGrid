//! Mode info card: automaton name, rule, boundary and description.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_mode_info(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        card.frame.show(ui, |ui| {
            let accent = self.theme.accent(&self.selected_mode);
            ui.label(
                egui::RichText::new(self.sim.automaton_name())
                    .strong()
                    .color(accent),
            );
            ui.label(
                egui::RichText::new(format!("Rule: {}", self.sim.rule_string()))
                    .monospace()
                    .small(),
            );
            if self.sim.supports_boundary() {
                ui.label(
                    egui::RichText::new(format!("Boundary: {}", self.sim.boundary().as_str()))
                        .small(),
                );
            } else {
                ui.label(egui::RichText::new("Boundary: fixed by mode").small());
            }
            ui.label(
                egui::RichText::new(automata::description(&self.selected_mode))
                    .small()
                    .weak(),
            );
        });

        ui.add_space(4.0);
    }
}
