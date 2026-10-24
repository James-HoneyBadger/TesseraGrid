//! Automaton card: mode, rule and boundary controls.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_automaton_section(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        let button_h = card.button_h;
        card.frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Automaton")
                .default_open(true)
                .show(ui, |ui| {
                    // Mode
                    ui.label("Mode:");
                    let old_mode = self.selected_mode.clone();
                    egui::ComboBox::from_id_salt("mode_combo")
                        .selected_text(&self.selected_mode)
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &m in automata::ALL_MODES {
                                ui.selectable_value(&mut self.selected_mode, m.to_owned(), m);
                            }
                        });
                    if self.selected_mode != old_mode {
                        self.rebuild_automaton();
                    }

                    // Pattern
                    ui.label("Pattern:");
                    let patterns: Vec<&str> = self.sim.available_patterns().to_vec();
                    let old_pat = self.selected_pattern.clone();
                    egui::ComboBox::from_id_salt("pattern_combo")
                        .selected_text(&self.selected_pattern)
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &p in &patterns {
                                ui.selectable_value(&mut self.selected_pattern, p.to_owned(), p);
                            }
                        });
                    if self.selected_pattern != old_pat {
                        self.do_reset();
                        let pat = self.selected_pattern.clone();
                        self.sim.load_pattern(&pat);
                    }
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Pattern Gallery…"),
                        )
                        .on_hover_text("Browse every pattern for this mode")
                        .clicked()
                    {
                        self.show_gallery = true;
                    }

                    // Boundary
                    ui.label("Boundary:");
                    ui.add_enabled_ui(self.sim.supports_boundary(), |ui| {
                        let old_bnd = self.selected_boundary;
                        egui::ComboBox::from_id_salt("boundary_combo")
                            .selected_text(self.selected_boundary.as_str())
                            .width(ui.available_width() - 4.0)
                            .show_ui(ui, |ui| {
                                for &b in BoundaryMode::all() {
                                    ui.selectable_value(&mut self.selected_boundary, b, b.as_str());
                                }
                            });
                        if self.selected_boundary != old_bnd {
                            self.sim.set_boundary(self.selected_boundary);
                        }
                    })
                    .response
                    .on_disabled_hover_text("This mode always wraps at the edges");

                    // Custom rule editor
                    if self.selected_mode == "Custom Rules" {
                        ui.label("Rule (B/S):");
                        let old_rule = self.custom_rule_str.clone();
                        ui.text_edit_singleline(&mut self.custom_rule_str);
                        if self.custom_rule_str != old_rule {
                            let (b, s) = LifeLike::parse_bs(&self.custom_rule_str);
                            self.custom_birth = b;
                            self.custom_survival = s;
                            self.rebuild_automaton();
                        }
                    }

                    // Seed + randomize
                    ui.add_space(4.0);
                    ui.label("Random seed:");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.seed_input)
                                .desired_width(ui.available_width() - 90.0),
                        );
                        if ui.button("Randomize").clicked() {
                            self.randomize();
                        }
                    });

                    // Resize button
                    ui.add_space(4.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Resize Grid..."),
                        )
                        .clicked()
                    {
                        let (gw, gh) = {
                            let g = self.sim.grid();
                            (g.width, g.height)
                        };
                        self.resize_w_input = gw.to_string();
                        self.resize_h_input = gh.to_string();
                        self.show_resize_dialog = true;
                    }

                    // RLE import from clipboard
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Import RLE"),
                        )
                        .on_hover_text("Paste RLE pattern from clipboard")
                        .clicked()
                    {
                        self.import_rle_from_clipboard();
                    }

                    // RLE export to clipboard
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Copy RLE"),
                        )
                        .on_hover_text("Copy current grid as RLE to clipboard")
                        .clicked()
                    {
                        self.copy_rle_to_clipboard();
                    }
                });
        });

        ui.add_space(4.0);
    }
}
