//! Color themes for the simulator UI and canvas.

use egui::Color32;

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

    pub fn is_dark(self) -> bool {
        !matches!(self, Self::Light | Self::Paper)
    }

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

    pub fn colors(self) -> ThemeColors {
        match self {
            Self::Dark => ThemeColors {
                window: rgb(22, 24, 30),
                panel: rgb(18, 20, 26),
                card: rgb(24, 27, 34),
                stroke: rgb(52, 58, 72),
                faint: rgb(30, 34, 42),
                extreme: rgb(12, 14, 20),
                grid: rgb(50, 50, 50),
                ok: rgb(100, 220, 100),
                cells: [
                    rgb(14, 14, 22),
                    rgb(220, 220, 255),
                    rgb(255, 80, 80),
                    rgb(255, 160, 0),
                    rgb(240, 240, 0),
                    rgb(0, 220, 100),
                    rgb(60, 140, 255),
                    rgb(200, 80, 255),
                    rgb(140, 140, 160),
                ],
            },
            Self::Light => ThemeColors {
                window: rgb(248, 249, 252),
                panel: rgb(241, 244, 249),
                card: rgb(246, 248, 252),
                stroke: rgb(208, 216, 228),
                faint: rgb(231, 236, 243),
                extreme: rgb(220, 226, 236),
                grid: rgb(210, 210, 210),
                ok: rgb(30, 142, 62),
                cells: [
                    Color32::WHITE,
                    Color32::BLACK,
                    rgb(200, 40, 40),
                    rgb(210, 110, 0),
                    rgb(180, 180, 0),
                    rgb(0, 150, 60),
                    rgb(0, 60, 200),
                    rgb(120, 0, 200),
                    rgb(80, 80, 80),
                ],
            },
            Self::Nord => ThemeColors {
                window: rgb(46, 52, 64),
                panel: rgb(41, 46, 57),
                card: rgb(52, 59, 73),
                stroke: rgb(76, 86, 106),
                faint: rgb(59, 66, 82),
                extreme: rgb(36, 41, 51),
                grid: rgb(67, 76, 94),
                ok: rgb(163, 190, 140),
                cells: [
                    rgb(46, 52, 64),
                    rgb(236, 239, 244),
                    rgb(191, 97, 106),
                    rgb(208, 135, 112),
                    rgb(235, 203, 139),
                    rgb(163, 190, 140),
                    rgb(129, 161, 193),
                    rgb(180, 142, 173),
                    rgb(122, 134, 153),
                ],
            },
            Self::SolarizedDark => ThemeColors {
                window: rgb(0, 43, 54),
                panel: rgb(7, 54, 66),
                card: rgb(7, 54, 66),
                stroke: rgb(88, 110, 117),
                faint: rgb(7, 54, 66),
                extreme: rgb(0, 31, 39),
                grid: rgb(15, 70, 82),
                ok: rgb(133, 153, 0),
                cells: [
                    rgb(0, 43, 54),
                    rgb(184, 196, 196),
                    rgb(220, 50, 47),
                    rgb(203, 75, 22),
                    rgb(181, 137, 0),
                    rgb(133, 153, 0),
                    rgb(38, 139, 210),
                    rgb(108, 113, 196),
                    rgb(88, 110, 117),
                ],
            },
            Self::Gruvbox => ThemeColors {
                window: rgb(40, 40, 40),
                panel: rgb(29, 32, 33),
                card: rgb(50, 48, 47),
                stroke: rgb(80, 73, 69),
                faint: rgb(60, 56, 54),
                extreme: rgb(29, 32, 33),
                grid: rgb(80, 73, 69),
                ok: rgb(184, 187, 38),
                cells: [
                    rgb(40, 40, 40),
                    rgb(235, 219, 178),
                    rgb(251, 73, 52),
                    rgb(254, 128, 25),
                    rgb(250, 189, 47),
                    rgb(184, 187, 38),
                    rgb(131, 165, 152),
                    rgb(211, 134, 155),
                    rgb(146, 131, 116),
                ],
            },
            Self::Dracula => ThemeColors {
                window: rgb(40, 42, 54),
                panel: rgb(33, 34, 44),
                card: rgb(47, 50, 65),
                stroke: rgb(68, 71, 90),
                faint: rgb(52, 55, 70),
                extreme: rgb(30, 31, 41),
                grid: rgb(68, 71, 90),
                ok: rgb(80, 250, 123),
                cells: [
                    rgb(40, 42, 54),
                    rgb(248, 248, 242),
                    rgb(255, 85, 85),
                    rgb(255, 184, 108),
                    rgb(241, 250, 140),
                    rgb(80, 250, 123),
                    rgb(139, 233, 253),
                    rgb(189, 147, 249),
                    rgb(98, 114, 164),
                ],
            },
            Self::HighContrast => ThemeColors {
                window: Color32::BLACK,
                panel: rgb(10, 10, 10),
                card: rgb(17, 17, 17),
                stroke: rgb(255, 255, 255),
                faint: rgb(26, 26, 26),
                extreme: Color32::BLACK,
                grid: rgb(102, 102, 102),
                ok: rgb(57, 255, 106),
                cells: [
                    Color32::BLACK,
                    Color32::WHITE,
                    rgb(255, 64, 64),
                    rgb(255, 159, 28),
                    rgb(255, 233, 59),
                    rgb(57, 255, 106),
                    rgb(77, 166, 255),
                    rgb(224, 102, 255),
                    rgb(191, 191, 191),
                ],
            },
            Self::Paper => ThemeColors {
                window: rgb(239, 230, 214),
                panel: rgb(243, 235, 221),
                card: rgb(246, 239, 226),
                stroke: rgb(214, 199, 174),
                faint: rgb(231, 220, 200),
                extreme: rgb(222, 208, 184),
                grid: rgb(216, 201, 176),
                ok: rgb(47, 122, 62),
                cells: [
                    rgb(246, 239, 226),
                    rgb(59, 50, 38),
                    rgb(166, 47, 47),
                    rgb(184, 92, 0),
                    rgb(138, 122, 0),
                    rgb(47, 122, 62),
                    rgb(36, 92, 158),
                    rgb(107, 63, 160),
                    rgb(110, 99, 85),
                ],
            },
        }
    }
}

/// Concrete colors for one theme.
pub struct ThemeColors {
    pub window: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub stroke: Color32,
    pub faint: Color32,
    pub extreme: Color32,
    pub grid: Color32,
    pub ok: Color32,
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
