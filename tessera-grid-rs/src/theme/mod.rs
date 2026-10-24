//! Color themes for the simulator UI and canvas.
//!
//! A theme defines the canvas palette (background plus states 1–8), grid and
//! outline colours, panel/card/bar colours, the success colour, and whether
//! egui starts from a dark or light base. [`Theme::accent`] adapts the
//! per-mode accent so titles and charts stay readable on light backgrounds.

use egui::Color32;

mod palettes;

/// The available color themes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Nord,
    SolarizedDark,
    Gruvbox,
    Dracula,
    HighContrast,
    Paper,
}

impl Theme {
    /// Every theme, in the order `T` cycles through them.
    pub fn all() -> &'static [Theme] {
        &[
            Self::Dark,
            Self::Light,
            Self::Nord,
            Self::SolarizedDark,
            Self::Gruvbox,
            Self::Dracula,
            Self::HighContrast,
            Self::Paper,
        ]
    }

    /// Human-readable name used in menus, config files and release docs.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Nord => "Nord",
            Self::SolarizedDark => "Solarized Dark",
            Self::Gruvbox => "Gruvbox",
            Self::Dracula => "Dracula",
            Self::HighContrast => "High Contrast",
            Self::Paper => "Paper",
        }
    }

    /// Parse a theme name (case-insensitive, a few aliases are accepted).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "nord" => Some(Self::Nord),
            "solarized dark" | "solarized-dark" | "solarized" => Some(Self::SolarizedDark),
            "gruvbox" => Some(Self::Gruvbox),
            "dracula" => Some(Self::Dracula),
            "high contrast" | "high-contrast" | "contrast" => Some(Self::HighContrast),
            "paper" | "sepia" => Some(Self::Paper),
            _ => None,
        }
    }

    /// Whether egui should start from its dark base style.
    pub fn is_dark(self) -> bool {
        !matches!(self, Self::Light | Self::Paper)
    }

    /// The next theme in cycle order (wraps around).
    pub fn next(self) -> Self {
        let all = Self::all();
        let index = all.iter().position(|&theme| theme == self).unwrap_or(0);
        all[(index + 1) % all.len()]
    }

    /// Mode accent adjusted for readability on this theme's background.
    pub fn accent(self, mode: &str) -> Color32 {
        let base = mode_accent_color(mode);
        if self.is_dark() {
            base
        } else {
            darken(base, 0.62)
        }
    }

    /// Concrete colors for this theme.
    pub fn colors(self) -> ThemeColors {
        palettes::colors(self)
    }
}

/// Concrete colors for one theme.
pub struct ThemeColors {
    /// Window background (`Visuals::window_fill`).
    pub window: Color32,
    /// Side-panel background.
    pub panel: Color32,
    /// Toolbar, status bar and card background.
    pub card: Color32,
    /// Borders and separators.
    pub stroke: Color32,
    /// Subtle widget backgrounds.
    pub faint: Color32,
    /// Text-edit and slider tracks.
    pub extreme: Color32,
    /// Canvas grid lines and border.
    pub grid: Color32,
    /// Transient success/status text.
    pub ok: Color32,
    /// Canvas palette: index 0 is the background, 1–8 are cell states.
    cells: [Color32; 9],
}

impl ThemeColors {
    /// Canvas background (state 0).
    pub fn background(&self) -> Color32 {
        self.cells[0]
    }

    /// Color for a cell state; states above 8 share the last color.
    pub fn cell(&self, state: u8) -> Color32 {
        self.cells
            .get(state as usize)
            .copied()
            .unwrap_or(self.cells[8])
    }
}

pub(crate) fn aging_color(base: Color32, age: u32) -> Color32 {
    let t = (age as f32 / 200.0).min(1.0);
    let warm = Color32::from_rgb(255, 180, 40);
    mix(base, warm, t)
}

pub(crate) fn mode_accent_color(mode: &str) -> Color32 {
    match mode {
        "Conway's Game of Life" => rgb(0, 180, 220),
        "High Life" => rgb(0, 210, 120),
        "Hexagonal Life" => rgb(160, 80, 240),
        "Immigration Game" => rgb(240, 160, 0),
        "Rainbow Game" => rgb(240, 60, 120),
        "Langton's Ant" => rgb(255, 120, 0),
        "Wireworld" => rgb(255, 210, 0),
        "Brian's Brain" => rgb(80, 140, 255),
        "Generations" => rgb(0, 220, 180),
        "Custom Rules" => rgb(180, 180, 180),
        _ => Color32::GRAY,
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn mix(from: Color32, to: Color32, t: f32) -> Color32 {
    let lerp = |a: u8, b: u8| (a as f32 * (1.0 - t) + b as f32 * t) as u8;
    Color32::from_rgb(
        lerp(from.r(), to.r()),
        lerp(from.g(), to.g()),
        lerp(from.b(), to.b()),
    )
}

fn darken(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (color.r() as f32 * factor) as u8,
        (color.g() as f32 * factor) as u8,
        (color.b() as f32 * factor) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color32) -> f32 {
        let channel = |c: u8| {
            let c = c as f32 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    #[test]
    fn names_round_trip_and_are_unique() {
        let mut names = std::collections::HashSet::new();
        for &theme in Theme::all() {
            assert!(
                names.insert(theme.name()),
                "duplicate name {}",
                theme.name()
            );
            assert_eq!(Theme::from_name(theme.name()), Some(theme));
        }
        assert_eq!(Theme::from_name("  NORD "), Some(Theme::Nord));
        assert_eq!(Theme::from_name("solarized"), Some(Theme::SolarizedDark));
        assert_eq!(Theme::from_name("dark"), Some(Theme::Dark));
        assert_eq!(Theme::from_name("nope"), None);
    }

    #[test]
    fn every_theme_has_readable_foreground() {
        for &theme in Theme::all() {
            let colors = theme.colors();
            let bg = luminance(colors.background());
            let alive = luminance(colors.cell(1));
            assert!(
                (bg - alive).abs() > 0.3,
                "{}: alive cell does not contrast with the background",
                theme.name()
            );
            assert_ne!(
                colors.background(),
                colors.cell(2),
                "{}: state 2 must differ from the background",
                theme.name()
            );
            assert_ne!(colors.grid, colors.background(), "{}: grid", theme.name());
        }
    }

    #[test]
    fn light_themes_darken_accents() {
        let dark = Theme::Dark.accent("Wireworld");
        let light = Theme::Light.accent("Wireworld");
        assert!(luminance(light) < luminance(dark));
        assert_eq!(Theme::Dark.next(), Theme::Light);
        assert_eq!(Theme::Paper.next(), Theme::Dark);
    }
}
