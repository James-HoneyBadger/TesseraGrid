//! Sidebar controls: mode info, automaton, view, statistics and export.

mod automaton;
mod export;
mod mode_info;
mod selection;
mod statistics;
mod view;

use super::*;

/// Styling shared by every sidebar card.
pub(super) struct SidebarCard {
    frame: egui::Frame,
    button_h: f32,
}

impl TesseraGridApp {
    pub(super) fn ui_controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = if self.compact_mode {
            Vec2::new(6.0, 6.0)
        } else {
            Vec2::new(8.0, 8.0)
        };

        let colors = self.theme.colors();
        let card_margin = if self.compact_mode {
            egui::Margin::same(8)
        } else {
            egui::Margin::same(10)
        };

        let card = SidebarCard {
            frame: egui::Frame::group(ui.style())
                .fill(colors.card)
                .stroke(egui::Stroke::new(1.0_f32, colors.stroke))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(card_margin),
            button_h: if self.compact_mode { 24.0 } else { 28.0 },
        };

        if self.tool_mode == ToolMode::Select {
            self.ui_selection_section(ui, &card);
        }

        self.ui_mode_info(ui, &card);
        self.ui_automaton_section(ui, &card);
        self.ui_view_section(ui, &card);
        self.ui_statistics_section(ui, &card);
        self.ui_export_section(ui, &card);
    }
}
