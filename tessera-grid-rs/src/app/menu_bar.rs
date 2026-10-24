//! Menu bar: file, edit, run, view and help menus.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open State…").clicked() {
                    ui.close();
                    self.load_state();
                }
                if ui.button("Save State…").clicked() {
                    ui.close();
                    self.save_state();
                }
                ui.separator();
                if ui.button("Export PNG…").clicked() {
                    ui.close();
                    self.export_png_dialog();
                }
                if ui.button("Export GIF…").clicked() {
                    ui.close();
                    self.export_gif_dialog();
                }
                ui.separator();
                if ui.button("Import RLE from Clipboard").clicked() {
                    ui.close();
                    self.import_rle_from_clipboard();
                }
                if ui.button("Copy RLE to Clipboard").clicked() {
                    ui.close();
                    self.copy_rle_to_clipboard();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Simulation", |ui| {
                let label = if self.running { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    self.running = !self.running;
                    if self.running {
                        self.last_step = Instant::now();
                    }
                    ui.close();
                }
                if ui.button("Step").clicked() {
                    self.push_undo();
                    self.do_step();
                    ui.close();
                }
                if ui.button("Run N steps…").clicked() {
                    self.show_run_n_dialog = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Reset").clicked() {
                    self.do_reset();
                    let pattern = self.selected_pattern.clone();
                    self.sim.load_pattern(&pattern);
                    ui.close();
                }
                if ui.button("Randomize").clicked() {
                    self.randomize();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(self.sim.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                    && self.sim.undo()
                {
                    self.running = false;
                    ui.close();
                }
                if ui
                    .add_enabled(self.sim.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                    && self.sim.redo()
                {
                    self.running = false;
                    ui.close();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Fit to window").clicked() {
                    self.pending_fit = true;
                    ui.close();
                }
                if ui.button("Toggle isometric 3-D").clicked() {
                    self.view_mode = match self.view_mode {
                        ViewMode::Flat => ViewMode::Isometric,
                        ViewMode::Isometric => ViewMode::Flat,
                    };
                    self.pending_fit = true;
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut self.show_grid, "Grid lines");
                ui.checkbox(&mut self.rounded_cells, "Rounded cells");
                ui.checkbox(&mut self.show_aging, "Cell aging overlay");
                ui.checkbox(&mut self.compact_mode, "Compact spacing");
                ui.menu_button("Theme", |ui| {
                    for &theme in Theme::all() {
                        if ui
                            .selectable_label(self.theme == theme, theme.name())
                            .clicked()
                        {
                            self.theme = theme;
                            ui.close();
                        }
                    }
                });
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard shortcuts").clicked() {
                    self.show_help = true;
                    ui.close();
                }
                if ui.button("Pattern gallery").clicked() {
                    self.show_gallery = true;
                    ui.close();
                }
                if ui.button("About Tessera Grid").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }
}
