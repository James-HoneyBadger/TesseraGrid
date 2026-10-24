//! Applies the selected [`crate::theme::Theme`] to egui's global style.

use super::*;

impl TesseraGridApp {
    pub(super) fn apply_theme(&mut self, ctx: &egui::Context) {
        let colors = self.theme.colors();
        let accent = self.theme.accent(&self.selected_mode);
        let key = (self.theme, self.compact_mode, accent);
        if self.theme_key == Some(key) {
            return;
        }
        self.theme_key = Some(key);

        ctx.set_theme(if self.theme.is_dark() {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });

        let mut style = (*ctx.global_style()).clone();
        {
            let visuals = &mut style.visuals;
            visuals.window_fill = colors.window;
            visuals.panel_fill = colors.panel;
            visuals.faint_bg_color = colors.faint;
            visuals.extreme_bg_color = colors.extreme;

            visuals.widgets.active.corner_radius = CornerRadius::same(6);
            visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
            visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
            visuals.widgets.open.corner_radius = CornerRadius::same(6);

            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, accent);
            visuals.selection.bg_fill =
                accent.linear_multiply(if self.theme.is_dark() { 0.25 } else { 0.18 });
            visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent);
        }

        if self.compact_mode {
            style.spacing.item_spacing = Vec2::new(6.0, 6.0);
            style.spacing.button_padding = Vec2::new(8.0, 5.0);
            style.spacing.interact_size.y = 22.0;
            style.spacing.slider_width = 110.0;
        } else {
            style.spacing.item_spacing = Vec2::new(8.0, 8.0);
            style.spacing.button_padding = Vec2::new(10.0, 6.0);
            style.spacing.interact_size.y = 26.0;
            style.spacing.slider_width = 130.0;
        }
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.indent = 14.0;
        style.visuals.window_corner_radius = CornerRadius::same(10);
        style.visuals.menu_corner_radius = CornerRadius::same(8);
        ctx.set_global_style(style);
    }
}
