use super::*;

impl TesseraGridApp {
    pub(super) fn save_config(&self) {
        let cfg = AppConfig {
            config_version: crate::core::config::CONFIG_VERSION,
            grid_width: self.sim.width(),
            grid_height: self.sim.height(),
            cell_size: self.cell_size,
            speed: self.speed,
            show_grid: self.show_grid,
            automaton_mode: self.selected_mode.clone(),
            pattern: self.selected_pattern.clone(),
            boundary: self.selected_boundary.as_str().to_owned(),
            custom_birth: self.custom_birth.clone(),
            custom_survival: self.custom_survival.clone(),
            theme: self.theme.name().to_owned(),
            dark_mode: self.theme.is_dark(),
            panel_width: self.panel_width,
            rounded_cells: self.rounded_cells,
            paint_state: self.paint_state,
            show_aging: self.show_aging,
            compact_mode: self.compact_mode,
            view_mode: match self.view_mode {
                ViewMode::Flat => "Flat".to_owned(),
                ViewMode::Isometric => "Isometric".to_owned(),
            },
            tool_mode: match self.tool_mode {
                ToolMode::Paint => "Paint".to_owned(),
                ToolMode::Select => "Select".to_owned(),
            },
            brush_size: self.brush_size,
            iso_height: self.iso_height,
        };
        if let Err(err) = cfg.save() {
            eprintln!("Tessera Grid: {err}");
        }
    }
}
