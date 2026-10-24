use serde::{Deserialize, Serialize};

/// Persistent application configuration (serialised to JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub grid_width: usize,
    pub grid_height: usize,
    pub cell_size: f32,
    pub speed: u32,
    pub show_grid: bool,
    pub automaton_mode: String,
    pub pattern: String,
    pub boundary: String,
    pub custom_birth: Vec<u8>,
    pub custom_survival: Vec<u8>,
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    #[serde(default = "default_panel_width")]
    pub panel_width: f32,
    #[serde(default)]
    pub rounded_cells: bool,
    #[serde(default = "default_paint_state")]
    pub paint_state: u8,
    #[serde(default)]
    pub show_aging: bool,
    #[serde(default)]
    pub compact_mode: bool,
}

fn default_true() -> bool {
    true
}
fn default_panel_width() -> f32 {
    220.0
}
fn default_paint_state() -> u8 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            grid_width: 100,
            grid_height: 100,
            cell_size: 8.0,
            speed: 10,
            show_grid: true,
            automaton_mode: "Conway's Game of Life".into(),
            pattern: "Glider".into(),
            boundary: "Wrap".into(),
            custom_birth: vec![3],
            custom_survival: vec![2, 3],
            dark_mode: true,
            panel_width: 220.0,
            rounded_cells: false,
            paint_state: 1,
            show_aging: false,
            compact_mode: false,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> std::path::PathBuf {
        dirs_next().join("lifegrid_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, data);
        }
    }
}

fn dirs_next() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".config")
        .join("lifegrid")
}
