use super::*;

impl LifeGridApp {
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
                if ui.button("About LifeGrid").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });
        });
    }

    pub(super) fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let accent = self.theme.accent(&self.selected_mode);

            ui.label(
                egui::RichText::new("LifeGrid")
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
