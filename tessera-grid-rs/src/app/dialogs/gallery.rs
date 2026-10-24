//! Pattern gallery dialog with thumbnails and pattern descriptions.

use crate::app::*;

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
    pub(crate) fn ui_gallery(&mut self, ctx: &egui::Context) {
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
}
