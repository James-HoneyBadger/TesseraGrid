use super::*;

/// Draw a 36×36 preview of a Conway pattern (or a placeholder).
fn pattern_thumbnail(ui: &mut egui::Ui, name: &str, theme: Theme) {
    let colors = theme.colors();
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), colors.background());
    let Some(points) = patterns::conway_pattern_points(name) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "·",
            egui::FontId::proportional(16.0),
            colors.cell(8),
        );
        return;
    };
    let min_x = points.iter().map(|p| p.0).min().unwrap_or(0);
    let max_x = points.iter().map(|p| p.0).max().unwrap_or(0);
    let min_y = points.iter().map(|p| p.1).min().unwrap_or(0);
    let max_y = points.iter().map(|p| p.1).max().unwrap_or(0);
    let span = (max_x - min_x + 1).max(max_y - min_y + 1) as f32;
    let cell = (rect.width() - 8.0) / span;
    for &(dx, dy) in points {
        let x = rect.left() + 4.0 + (dx - min_x) as f32 * cell;
        let y = rect.top() + 4.0 + (dy - min_y) as f32 * cell;
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(x, y), Vec2::splat(cell.max(1.5))),
            CornerRadius::ZERO,
            colors.cell(1),
        );
    }
}

impl TesseraGridApp {
    pub(super) fn ui_about(&mut self, ctx: &egui::Context) {
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

    pub(super) fn ui_gallery(&mut self, ctx: &egui::Context) {
        if !self.show_gallery {
            return;
        }
        let mut open = true;
        egui::Window::new("Pattern Gallery")
            .open(&mut open)
            .default_width(520.0)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.gallery_filter)
                            .hint_text("filter patterns")
                            .desired_width(200.0),
                    );
                    if ui.button("Clear").clicked() {
                        self.gallery_filter.clear();
                    }
                });
                ui.separator();

                let filter = self.gallery_filter.to_lowercase();
                let patterns: Vec<&'static str> = self.sim.available_patterns().to_vec();
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        egui::Grid::new("gallery_grid")
                            .num_columns(3)
                            .spacing([12.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                for name in patterns {
                                    if !filter.is_empty() && !name.to_lowercase().contains(&filter)
                                    {
                                        continue;
                                    }
                                    pattern_thumbnail(ui, name, self.theme);
                                    let selected = name == self.selected_pattern;
                                    if ui.selectable_label(selected, name).clicked() {
                                        self.selected_pattern = name.to_owned();
                                        self.do_reset();
                                        self.sim.load_pattern(name);
                                        self.pending_fit = true;
                                        self.show_gallery = false;
                                    }
                                    let description = match name {
                                        "Random Soup" => "Random 15% soup",
                                        "Empty" => "Clears the grid",
                                        _ => {
                                            patterns::conway_pattern_description(name).unwrap_or("")
                                        }
                                    };
                                    ui.label(egui::RichText::new(description).small().weak());
                                    ui.end_row();
                                }
                            });
                    });

                ui.separator();
                ui.label(
                    egui::RichText::new(format!(
                        "{} patterns available in {}",
                        self.sim.available_patterns().len(),
                        self.sim.automaton_name()
                    ))
                    .small()
                    .weak(),
                );
            });
        if !open {
            self.show_gallery = false;
        }
    }

    pub(super) fn ui_help_modal(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (key, desc) in SHORTCUTS {
                            ui.label(egui::RichText::new(*key).monospace().strong());
                            ui.label(*desc);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    self.show_help = false;
                }
            });
        if !open {
            self.show_help = false;
        }
    }

    pub(super) fn ui_resize_dialog(&mut self, ctx: &egui::Context) {
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
                    egui::RichText::new("New grids are clamped to 10–1000 cells.")
                        .small()
                        .weak(),
                );
                ui.horizontal(|ui| {
                    if ui.button("Resize").clicked() {
                        let w = self
                            .resize_w_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        let old_ratio = self.sim.height() as f32 / self.sim.width().max(1) as f32;
                        let mut h = self
                            .resize_h_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        if self.resize_aspect {
                            h = ((w as f32 * old_ratio).round() as usize).clamp(10, 1000);
                        }
                        let preserve = self.resize_preserve;
                        self.sim.resize(
                            w,
                            h,
                            RuleSpec::new(
                                self.selected_boundary,
                                &self.custom_birth,
                                &self.custom_survival,
                            ),
                            preserve,
                        );
                        if !preserve {
                            self.sim.load_pattern(&self.selected_pattern);
                        }
                        self.running = false;
                        self.viewport_offset = Vec2::ZERO;
                        self.pending_fit = true;
                        self.set_status(format!("Grid resized to {}×{}", w, h));
                        self.show_resize_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_resize_dialog = false;
                    }
                });
            });
        if !open {
            self.show_resize_dialog = false;
        }
    }

    pub(super) fn ui_run_n_dialog(&mut self, ctx: &egui::Context) {
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
