use super::*;

pub(crate) fn aging_color(base: Color32, age: u32) -> Color32 {
    let t = (age as f32 / 200.0).min(1.0);
    let warm = Color32::from_rgb(255, 180, 40);
    let r = (base.r() as f32 * (1.0 - t) + warm.r() as f32 * t) as u8;
    let g = (base.g() as f32 * (1.0 - t) + warm.g() as f32 * t) as u8;
    let b = (base.b() as f32 * (1.0 - t) + warm.b() as f32 * t) as u8;
    Color32::from_rgb(r, g, b)
}

pub(crate) fn cell_color(state: u8, dark: bool) -> Color32 {
    if dark {
        match state {
            0 => Color32::from_rgb(14, 14, 22),
            1 => Color32::from_rgb(220, 220, 255),
            2 => Color32::from_rgb(255, 80, 80),
            3 => Color32::from_rgb(255, 160, 0),
            4 => Color32::from_rgb(240, 240, 0),
            5 => Color32::from_rgb(0, 220, 100),
            6 => Color32::from_rgb(60, 140, 255),
            7 => Color32::from_rgb(200, 80, 255),
            8 => Color32::from_rgb(140, 140, 160),
            _ => Color32::from_rgb(100, 100, 120),
        }
    } else {
        match state {
            0 => Color32::WHITE,
            1 => Color32::BLACK,
            2 => Color32::from_rgb(200, 40, 40),
            3 => Color32::from_rgb(210, 110, 0),
            4 => Color32::from_rgb(180, 180, 0),
            5 => Color32::from_rgb(0, 150, 60),
            6 => Color32::from_rgb(0, 60, 200),
            7 => Color32::from_rgb(120, 0, 200),
            8 => Color32::from_rgb(80, 80, 80),
            _ => Color32::from_rgb(120, 120, 120),
        }
    }
}

pub(crate) fn mode_accent_color(mode: &str) -> Color32 {
    match mode {
        "Conway's Game of Life" => Color32::from_rgb(0, 180, 220),
        "High Life" => Color32::from_rgb(0, 210, 120),
        "Hexagonal Life" => Color32::from_rgb(160, 80, 240),
        "Immigration Game" => Color32::from_rgb(240, 160, 0),
        "Rainbow Game" => Color32::from_rgb(240, 60, 120),
        "Langton's Ant" => Color32::from_rgb(255, 120, 0),
        "Wireworld" => Color32::from_rgb(255, 210, 0),
        "Brian's Brain" => Color32::from_rgb(80, 140, 255),
        "Generations" => Color32::from_rgb(0, 220, 180),
        "Custom Rules" => Color32::from_rgb(180, 180, 180),
        _ => Color32::GRAY,
    }
}

impl LifeGridApp {
    pub(super) fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });

        let accent = mode_accent_color(&self.selected_mode);
        let mut style = (*ctx.global_style()).clone();
        {
            let visuals = &mut style.visuals;
            if self.dark_mode {
                visuals.window_fill = Color32::from_rgb(22, 24, 30);
                visuals.panel_fill = Color32::from_rgb(18, 20, 26);
                visuals.faint_bg_color = Color32::from_rgb(30, 34, 42);
                visuals.extreme_bg_color = Color32::from_rgb(12, 14, 20);
            } else {
                visuals.window_fill = Color32::from_rgb(248, 249, 252);
                visuals.panel_fill = Color32::from_rgb(241, 244, 249);
                visuals.faint_bg_color = Color32::from_rgb(231, 236, 243);
                visuals.extreme_bg_color = Color32::from_rgb(220, 226, 236);
            }

            visuals.widgets.active.corner_radius = CornerRadius::same(6);
            visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
            visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
            visuals.widgets.open.corner_radius = CornerRadius::same(6);

            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, accent);
            visuals.selection.bg_fill = accent.linear_multiply(0.25);
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
