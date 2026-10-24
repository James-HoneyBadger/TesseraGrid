use super::*;

impl LifeGridApp {
    pub(super) fn ui_controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = if self.compact_mode {
            Vec2::new(6.0, 6.0)
        } else {
            Vec2::new(8.0, 8.0)
        };

        let card_fill = if self.dark_mode {
            Color32::from_rgb(24, 27, 34)
        } else {
            Color32::from_rgb(246, 248, 252)
        };
        let card_stroke = if self.dark_mode {
            Color32::from_rgb(52, 58, 72)
        } else {
            Color32::from_rgb(208, 216, 228)
        };
        let card_margin = if self.compact_mode {
            egui::Margin::same(8)
        } else {
            egui::Margin::same(10)
        };
        let button_h = if self.compact_mode { 24.0 } else { 28.0 };

        let section_frame = egui::Frame::group(ui.style())
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, card_stroke))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(card_margin);

        // ── Selection panel (visible only in Select mode) ────────────────────
        if self.tool_mode == ToolMode::Select {
            section_frame.show(ui, |ui| {
                egui::CollapsingHeader::new("Selection")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some((x0, y0, x1, y1)) = self.selection {
                            let sw = x1 - x0 + 1;
                            let sh = y1 - y0 + 1;
                            ui.label(format!("{}×{}  at  ({}, {})", sw, sh, x0, y0));
                            ui.horizontal(|ui| {
                                let w = (ui.available_width() - 12.0) / 3.0;
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Copy"))
                                    .on_hover_text("Ctrl+C")
                                    .clicked()
                                {
                                    self.copy_selection();
                                }
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Cut"))
                                    .on_hover_text("Ctrl+X")
                                    .clicked()
                                {
                                    self.cut_selection();
                                }
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Clear"))
                                    .on_hover_text("Delete")
                                    .clicked()
                                {
                                    self.clear_selection_cells();
                                }
                            });
                            if ui
                                .add_sized(
                                    [ui.available_width(), button_h],
                                    egui::Button::new("Deselect"),
                                )
                                .on_hover_text("Escape")
                                .clicked()
                            {
                                self.selection = None;
                            }
                        } else {
                            ui.weak("Drag on canvas to select a region.");
                        }
                        if let Some((_, cw, ch)) = &self.clipboard {
                            ui.separator();
                            ui.label(format!("Clipboard: {}×{}", cw, ch));
                            if ui
                                .add_sized(
                                    [ui.available_width(), button_h],
                                    egui::Button::new("Paste"),
                                )
                                .on_hover_text("Ctrl+V")
                                .clicked()
                            {
                                self.paste_clipboard();
                            }
                        }
                    });
            });
            ui.add_space(4.0);
        }

        // ── Mode info ────────────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            let accent = mode_accent_color(&self.selected_mode);
            ui.horizontal(|ui| {
                ui.colored_label(accent, "●");
                ui.label(egui::RichText::new(self.sim.automaton_name()).strong());
            });
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

        // ── Automaton section ────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
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

        // ── View section ─────────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("View")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("Speed: {} steps/s", self.speed));
                    ui.add(egui::Slider::new(&mut self.speed, 1..=200).show_value(false));

                    ui.label(format!("Cell size: {}px", self.cell_size as u32));
                    ui.horizontal(|ui| {
                        if ui.small_button("−").clicked() {
                            self.cell_size = (self.cell_size - 1.0).max(1.0);
                        }
                        ui.add(
                            egui::Slider::new(&mut self.cell_size, 1.0..=64.0).show_value(false),
                        );
                        if ui.small_button("+").clicked() {
                            self.cell_size = (self.cell_size + 1.0).min(64.0);
                        }
                    });

                    ui.checkbox(&mut self.show_grid, "Grid lines (G)");
                    ui.checkbox(&mut self.rounded_cells, "Rounded cells");
                    ui.checkbox(&mut self.show_aging, "Cell aging overlay");

                    if self.view_mode == ViewMode::Isometric {
                        ui.add_space(4.0);
                        ui.label(format!("Block height: {:.2}×", self.iso_height));
                        ui.add(
                            egui::Slider::new(&mut self.iso_height, 0.25..=4.0).show_value(false),
                        );
                    }

                    ui.add_space(4.0);
                    let max_state = self.sim.max_state().max(1);
                    self.paint_state = self.paint_state.clamp(1, max_state);
                    ui.label(format!("Paint state (1–{max_state}):"));
                    ui.add(
                        egui::Slider::new(&mut self.paint_state, 1..=max_state)
                            .clamping(egui::SliderClamping::Always),
                    );
                    ui.label(format!(
                        "Brush size: {}×{}",
                        self.brush_size, self.brush_size
                    ));
                    ui.add(egui::Slider::new(&mut self.brush_size, 1..=8));
                    ui.label(
                        egui::RichText::new("Left-click = paint   Right-click = erase")
                            .small()
                            .weak(),
                    );
                });
        });

        ui.add_space(4.0);

        // ── Statistics section ───────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Statistics")
                .default_open(true)
                .show(ui, |ui| {
                    let history = self.sim.pop_history();
                    if history.len() > 1 {
                        let max_pop = history.iter().copied().max().unwrap_or(1).max(1);
                        let min_pop = history.iter().copied().min().unwrap_or(0);
                        let avg_pop = history.iter().sum::<u64>() / history.len() as u64;
                        let current = history.back().copied().unwrap_or(0);

                        ui.label(format!(
                            "Now {current}   Peak {max_pop}   Min {min_pop}   Avg {avg_pop}"
                        ));

                        let points: Vec<[f64; 2]> = history
                            .iter()
                            .enumerate()
                            .map(|(i, &pop)| [i as f64, pop as f64])
                            .collect();
                        egui_plot::Plot::new("population_plot")
                            .height(96.0)
                            .allow_drag(false)
                            .allow_zoom(false)
                            .allow_scroll(false)
                            .allow_boxed_zoom(false)
                            .include_y(0.0)
                            .x_axis_label("generation")
                            .y_axis_label("population")
                            .show(ui, |plot_ui| {
                                plot_ui.line(
                                    egui_plot::Line::new("population", points)
                                        .color(mode_accent_color(&self.selected_mode))
                                        .width(1.5),
                                );
                            });
                    } else {
                        ui.weak("Run the simulation to see statistics.");
                    }

                    ui.horizontal(|ui| {
                        if ui.small_button("Reset stats").clicked() {
                            self.sim.clear_pop_history();
                        }
                        if ui
                            .add_enabled(
                                !self.sim.pop_history().is_empty(),
                                egui::Button::new("Export CSV…").small(),
                            )
                            .on_hover_text("Save the population history as CSV")
                            .clicked()
                        {
                            self.export_stats_csv();
                        }
                    });
                });
        });

        ui.add_space(4.0);

        // ── Export section ───────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
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
                            .on_hover_text("Save current grid to a .lifegrid file")
                            .clicked()
                        {
                            self.save_state();
                        }
                        if ui
                            .add_sized([w, button_h], egui::Button::new("Load State..."))
                            .on_hover_text("Load a previously saved .lifegrid file")
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
