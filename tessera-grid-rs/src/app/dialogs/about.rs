//! About dialog: version, links and credits.

use crate::app::*;

impl TesseraGridApp {
    pub(crate) fn ui_about(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }
        let mut open = true;
        egui::Window::new("About Tessera Grid")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading("Tessera Grid");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("A fast, native cellular-automaton simulator built with Rust and egui.");
                ui.label("MIT licensed.");
                ui.label(egui::RichText::new("Formerly LifeGrid").small().weak());
                ui.hyperlink_to(
                    "github.com/James-HoneyBadger/TesseraGrid",
                    "https://github.com/James-HoneyBadger/TesseraGrid",
                );
                ui.separator();
                ui.label(format!(
                    "{} modes · {}×{} grid · rule {}",
                    automata::ALL_MODES.len(),
                    self.sim.width(),
                    self.sim.height(),
                    self.sim.rule_string()
                ));
                ui.add_space(6.0);
                if ui.button("Close").clicked() {
                    self.show_about = false;
                }
            });
        if !open {
            self.show_about = false;
        }
    }
}
