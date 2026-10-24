//! Selection card: copy, cut, clear and paste for the current marquee.

use super::*;

impl TesseraGridApp {
    pub(super) fn ui_selection_section(&mut self, ui: &mut egui::Ui, card: &SidebarCard) {
        let button_h = card.button_h;
        card.frame.show(ui, |ui| {
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
                            .add_sized([ui.available_width(), button_h], egui::Button::new("Paste"))
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
}
