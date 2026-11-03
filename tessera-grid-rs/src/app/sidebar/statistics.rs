//! Statistics card: live population, generation and chart.

use super::*;

impl TesseraGridApp {
    /// Metrics for the current generation, throttled so a running simulation
    /// does not pay for an entropy pass on every rendered frame.
    fn current_metrics(&mut self) -> crate::core::analysis::Metrics {
        let revision = self.sim.revision();
        if let Some((computed_at, cached_revision, metrics)) = self.metrics_cache {
            let fresh_enough = self.running && computed_at.elapsed() < Duration::from_millis(200);
            if cached_revision == revision || fresh_enough {
                return metrics;
            }
        }
        let metrics = self.sim.metrics();
        self.metrics_cache = Some((Instant::now(), revision, metrics));
        metrics
    }

    pub(super) fn ui_statistics_section(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        card.frame.show(ui, |ui| {
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

                        let first_generation = self
                            .sim
                            .generation
                            .saturating_sub(history.len().saturating_sub(1) as u64);
                        let points: Vec<[f64; 2]> = history
                            .iter()
                            .enumerate()
                            .map(|(i, &pop)| [(first_generation + i as u64) as f64, pop as f64])
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
                                        .color(self.theme.accent(&self.selected_mode))
                                        .width(1.5),
                                );
                            });
                    } else {
                        ui.weak("Run the simulation to see statistics.");
                    }

                    let metrics = self.current_metrics();
                    ui.separator();
                    ui.label(format!("Entropy {:.3} bits/cell", metrics.entropy))
                        .on_hover_text("Shannon state-frequency entropy; not spatial complexity or a chaos classifier.");
                    if self.sim.mode() == "Elementary 1D" {
                        ui.weak("Active row only; diagram history is excluded. Bounds use y=0.");
                    }
                    if let Some((x0, y0, x1, y1)) = metrics.bbox {
                        ui.label(format!(
                            "Bounds {}×{} at ({x0}, {y0})",
                            x1 - x0 + 1,
                            y1 - y0 + 1
                        ));
                    }
                    match metrics.cycle {
                        Some(cycle) => ui.label(format!(
                            "Cycle: period {} (first retained at gen {})",
                            cycle.period, cycle.transient
                        )),
                        None => ui.weak("Cycle: none observed in retained state window"),
                    };

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
                        if ui
                            .small_button("Analysis…")
                            .on_hover_text("Sensitivity experiment and rule sweep")
                            .clicked()
                        {
                            self.show_analysis = true;
                        }
                    });
                });
        });

        ui.add_space(4.0);
    }
}
