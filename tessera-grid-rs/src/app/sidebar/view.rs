//! View card: theme, rendering and playback preferences.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_view_section(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        card.frame.show(ui, |ui| {
            egui::CollapsingHeader::new("View")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label("Theme:");
                    egui::ComboBox::from_id_salt("theme_combo")
                        .selected_text(self.theme.name())
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &theme in Theme::all() {
                                ui.selectable_value(&mut self.theme, theme, theme.name());
                            }
                        });

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
                    if max_state > 1 {
                        ui.label(format!("Paint state (1–{max_state}):"));
                        ui.add(
                            egui::Slider::new(&mut self.paint_state, 1..=max_state)
                                .clamping(egui::SliderClamping::Always),
                        );
                    }
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
    }
}
