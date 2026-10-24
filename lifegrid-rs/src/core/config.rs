use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::Theme;

/// Current config schema version. Bump when a migration is required.
///
/// v2 added the `theme` field; `dark_mode` is kept for older builds.
pub const CONFIG_VERSION: u32 = 2;

/// Persistent application configuration (serialised to JSON).
///
/// Every field has a serde default so that older config files keep loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_config_version")]
    pub config_version: u32,
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
    /// Legacy boolean theme flag, kept so older builds and configs interoperate.
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    /// Theme name (see `Theme::all`). Empty in pre-v2 configs and resolved from
    /// `dark_mode` during sanitisation.
    #[serde(default)]
    pub theme: String,
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
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
    #[serde(default = "default_tool_mode")]
    pub tool_mode: String,
    #[serde(default = "default_brush_size")]
    pub brush_size: u8,
    #[serde(default = "default_iso_height")]
    pub iso_height: f32,
}

fn default_config_version() -> u32 {
    CONFIG_VERSION
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
fn default_view_mode() -> String {
    "Flat".to_owned()
}
fn default_tool_mode() -> String {
    "Paint".to_owned()
}
fn default_brush_size() -> u8 {
    1
}
fn default_iso_height() -> f32 {
    1.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
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
            theme: "Dark".to_owned(),
            panel_width: 220.0,
            rounded_cells: false,
            paint_state: 1,
            show_aging: false,
            compact_mode: false,
            view_mode: "Flat".into(),
            tool_mode: "Paint".into(),
            brush_size: 1,
            iso_height: 1.0,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        config_dir().join("lifegrid_config.json")
    }

    /// Load the config, returning a human-readable warning when a file exists
    /// but cannot be parsed (the defaults are used in that case).
    pub fn try_load() -> (Self, Option<String>) {
        let path = Self::config_path();
        let Ok(data) = std::fs::read_to_string(&path) else {
            return (Self::default(), None);
        };
        match serde_json::from_str::<Self>(&data) {
            Ok(mut cfg) => {
                cfg.sanitize();
                (cfg, None)
            }
            Err(err) => (
                Self::default(),
                Some(format!(
                    "Config at {} is invalid ({err}); using defaults",
                    path.display()
                )),
            ),
        }
    }

    /// Clamp values into the ranges the UI supports.
    pub fn sanitize(&mut self) {
        self.grid_width = self.grid_width.clamp(10, 1000);
        self.grid_height = self.grid_height.clamp(10, 1000);
        self.cell_size = self.cell_size.clamp(1.0, 64.0);
        self.speed = self.speed.clamp(1, 200);
        self.panel_width = self.panel_width.clamp(160.0, 420.0);
        self.paint_state = self.paint_state.clamp(1, 24);
        self.brush_size = self.brush_size.clamp(1, 8);
        self.iso_height = self.iso_height.clamp(0.25, 4.0);

        let theme = Theme::from_name(&self.theme).unwrap_or(if self.dark_mode {
            Theme::Dark
        } else {
            Theme::Light
        });
        self.theme = theme.name().to_owned();
        self.dark_mode = theme.is_dark();

        self.config_version = CONFIG_VERSION;
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Cannot serialise config: {e}"))?;
        std::fs::write(&path, data).map_err(|e| format!("Cannot write {}: {e}", path.display()))
    }
}

fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("LIFEGRID_CONFIG_DIR") {
        return PathBuf::from(dir);
    }

    #[cfg(target_os = "windows")]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("lifegrid");
    }

    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("lifegrid");
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        #[cfg(target_os = "macos")]
        {
            return home
                .join("Library")
                .join("Application Support")
                .join("lifegrid");
        }
        #[cfg(not(target_os = "macos"))]
        {
            return home.join(".config").join("lifegrid");
        }
    }

    PathBuf::from(".lifegrid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        let cfg: AppConfig =
            serde_json::from_str(r#"{"grid_width":10,"grid_height":10,"cell_size":4.0,"speed":5,"show_grid":true,"automaton_mode":"High Life","pattern":"Random Soup","boundary":"Wrap","custom_birth":[3],"custom_survival":[2,3]}"#)
                .expect("deserialize");
        assert_eq!(cfg.config_version, CONFIG_VERSION);
        assert_eq!(cfg.brush_size, 1);
        assert_eq!(cfg.view_mode, "Flat");
        assert!(cfg.dark_mode);
        assert!(cfg.theme.is_empty(), "old configs omit the theme");
    }

    #[test]
    fn theme_migrates_from_legacy_dark_mode() {
        let mut cfg: AppConfig = serde_json::from_str(
            r#"{"grid_width":10,"grid_height":10,"cell_size":4.0,"speed":5,"show_grid":true,"automaton_mode":"High Life","pattern":"Random Soup","boundary":"Wrap","custom_birth":[3],"custom_survival":[2,3],"dark_mode":false}"#,
        )
        .expect("deserialize");
        cfg.sanitize();
        assert_eq!(cfg.theme, "Light");
        assert!(!cfg.dark_mode);
    }

    #[test]
    fn explicit_theme_wins_and_is_normalised() {
        let mut cfg = AppConfig {
            theme: "  nord ".to_owned(),
            dark_mode: false,
            ..AppConfig::default()
        };
        cfg.sanitize();
        assert_eq!(cfg.theme, "Nord");
        assert!(cfg.dark_mode);

        cfg.theme = "not-a-theme".to_owned();
        cfg.sanitize();
        assert_eq!(cfg.theme, "Dark", "invalid names fall back to dark_mode");
    }

    #[test]
    fn sanitize_clamps_out_of_range_values() {
        let mut cfg = AppConfig {
            grid_width: 5,
            speed: 9_999,
            brush_size: 200,
            panel_width: 5.0,
            ..AppConfig::default()
        };
        cfg.sanitize();
        assert_eq!(cfg.grid_width, 10);
        assert_eq!(cfg.speed, 200);
        assert_eq!(cfg.brush_size, 8);
        assert_eq!(cfg.panel_width, 160.0);
    }
}
