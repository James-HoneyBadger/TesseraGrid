//! Statistics card: live population, generation and chart.

use super::*;

impl TesseraGridApp {
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
    }
}
