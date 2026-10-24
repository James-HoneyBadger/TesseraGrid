//! Built-in Conway patterns used by the loader and the pattern gallery.

use crate::core::Grid;

/// A named set of (dx, dy) offsets centred at the grid midpoint.
pub struct PatternDef {
    pub name: &'static str,
    pub points: &'static [(i32, i32)],
    pub description: &'static str,
}

/// All Conway patterns available in the application.
pub const CONWAY_PATTERN_NAMES: &[&str] = &[
    "Glider",
    "Blinker",
    "Toad",
    "Beacon",
    "Block",
    "Beehive",
    "Loaf",
    "Boat",
    "LWSS",
    "MWSS",
    "Glider Gun",
    "Acorn",
    "R-Pentomino",
    "Pulsar",
    "Random Soup",
    "Empty",
];

/// Return the coordinate offsets for a named Conway pattern, or `None`.
pub fn conway_pattern_points(name: &str) -> Option<&'static [(i32, i32)]> {
    CONWAY_PATTERNS
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.points)
}

/// Description for a named Conway pattern.
pub fn conway_pattern_description(name: &str) -> Option<&'static str> {
    CONWAY_PATTERNS
        .iter()
        .find(|p| p.name == name)
        .map(|p| p.description)
}

static CONWAY_PATTERNS: &[PatternDef] = &[
    PatternDef {
        name: "Glider",
        points: &[(1, -1), (2, 0), (0, 1), (1, 1), (2, 1)],
        description: "Classic glider – travels diagonally.",
    },
    PatternDef {
        name: "Blinker",
        points: &[(-1, 0), (0, 0), (1, 0)],
        description: "Period-2 oscillator.",
    },
    PatternDef {
        name: "Toad",
        points: &[(0, 0), (1, 0), (2, 0), (-1, 1), (0, 1), (1, 1)],
        description: "Period-2 oscillator.",
    },
    PatternDef {
        name: "Beacon",
        points: &[(-2, -2), (-1, -2), (-2, -1), (0, 0), (1, 0), (0, 1), (1, 1)],
        description: "Period-2 oscillator.",
    },
    PatternDef {
        name: "Block",
        points: &[(0, 0), (1, 0), (0, 1), (1, 1)],
        description: "Still life – 2×2 square.",
    },
    PatternDef {
        name: "Beehive",
        points: &[(1, -1), (2, -1), (0, 0), (3, 0), (1, 1), (2, 1)],
        description: "Still life.",
    },
    PatternDef {
        name: "Loaf",
        points: &[(1, -2), (2, -2), (0, -1), (3, -1), (1, 0), (3, 0), (2, 1)],
        description: "Still life.",
    },
    PatternDef {
        name: "Boat",
        points: &[(0, -1), (1, -1), (0, 0), (2, 0), (1, 1)],
        description: "Still life.",
    },
    PatternDef {
        name: "LWSS",
        points: &[
            (1, -2),
            (4, -2),
            (0, -1),
            (0, 0),
            (4, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
        ],
        description: "Lightweight spaceship.",
    },
    PatternDef {
        name: "MWSS",
        points: &[
            (2, -3),
            (1, -2),
            (5, -2),
            (0, -1),
            (0, 0),
            (5, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
        ],
        description: "Middleweight spaceship.",
    },
    PatternDef {
        name: "R-Pentomino",
        points: &[(1, -1), (2, -1), (0, 0), (1, 0), (1, 1)],
        description: "Methuselah that runs for 1103 generations.",
    },
    PatternDef {
        name: "Acorn",
        points: &[(1, -2), (3, -1), (0, 0), (1, 0), (4, 0), (5, 0), (6, 0)],
        description: "Methuselah – 5206 generations.",
    },
    PatternDef {
        name: "Pulsar",
        points: &[
            // Top-left quadrant (mirrored ×4)
            (-6, -4),
            (-5, -4),
            (-4, -4),
            (-6, -3),
            (-3, -3),
            (-6, -2),
            (-3, -2),
            (-5, -1),
            (-4, -1),
            (-3, -1),
            (-6, 4),
            (-5, 4),
            (-4, 4),
            (-6, 3),
            (-3, 3),
            (-6, 2),
            (-3, 2),
            (-5, 1),
            (-4, 1),
            (-3, 1),
            (6, -4),
            (5, -4),
            (4, -4),
            (6, -3),
            (3, -3),
            (6, -2),
            (3, -2),
            (5, -1),
            (4, -1),
            (3, -1),
            (6, 4),
            (5, 4),
            (4, 4),
            (6, 3),
            (3, 3),
            (6, 2),
            (3, 2),
            (5, 1),
            (4, 1),
            (3, 1),
        ],
        description: "Period-3 oscillator.",
    },
    PatternDef {
        name: "Glider Gun",
        // Gosper Glider Gun – offsets relative to centre
        points: &[
            (-17, -4),
            (-17, -3),
            (-16, -3),
            (-15, -2),
            (-14, -2),
            (-21, -2),
            (-22, -2),
            (-15, -1),
            (-14, -1),
            (-21, -1),
            (-22, -1),
            (-13, 0),
            (-12, 0),
            (-15, 1),
            (-14, 1),
            (-15, 2),
            (-14, 2),
            (-13, 3),
            (-12, 3),
            (-6, -4),
            (-5, -4),
            (-6, -3),
            (-5, -3),
            (-6, -2),
            (-5, -2),
            (-7, -1),
            (-4, -1),
            (-8, 0),
            (-3, 0),
            (-8, 1),
            (-3, 1),
            (-7, 2),
            (-4, 2),
            (2, -4),
            (2, -3),
            (2, -2),
            (3, -4),
            (3, -2),
            (4, -3),
        ],
        description: "Gosper Glider Gun – first known infinite-growth pattern.",
    },
];

// ---------------------------------------------------------------------------
// Draft patterns (in-app editor)
// ---------------------------------------------------------------------------

/// A square, editable pattern canvas used by the pattern editor.
///
/// Cells are stored row-major; `true` means alive. The draft is independent of
/// the simulation, so edits can be previewed before being stamped into a grid.
#[derive(Clone)]
pub struct PatternDraft {
    size: usize,
    cells: Vec<bool>,
}

impl PatternDraft {
    /// Create an empty `size × size` draft.
    pub fn new(size: usize) -> Self {
        let size = size.max(1);
        Self {
            size,
            cells: vec![false; size * size],
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.cells[y * self.size + x]
    }

    pub fn set(&mut self, x: usize, y: usize, alive: bool) {
        if x < self.size && y < self.size {
            self.cells[y * self.size + x] = alive;
        }
    }

    pub fn population(&self) -> usize {
        self.cells.iter().filter(|&&alive| alive).count()
    }

    pub fn is_empty(&self) -> bool {
        !self.cells.iter().any(|&alive| alive)
    }

    pub fn clear(&mut self) {
        self.cells.fill(false);
    }

    pub fn invert(&mut self) {
        for cell in &mut self.cells {
            *cell = !*cell;
        }
    }

    /// Rotate the draft 90° clockwise.
    pub fn rotate_cw(&mut self) {
        let n = self.size;
        let mut rotated = vec![false; n * n];
        for y in 0..n {
            for x in 0..n {
                rotated[x * n + (n - 1 - y)] = self.get(x, y);
            }
        }
        self.cells = rotated;
    }

    pub fn flip_horizontal(&mut self) {
        let n = self.size;
        for y in 0..n {
            for x in 0..n / 2 {
                self.cells.swap(y * n + x, y * n + (n - 1 - x));
            }
        }
    }

    pub fn flip_vertical(&mut self) {
        let n = self.size;
        for y in 0..n / 2 {
            for x in 0..n {
                self.cells.swap(y * n + x, (n - 1 - y) * n + x);
            }
        }
    }

    /// Copy the draft into a standalone grid so it can be exported as RLE.
    pub fn to_grid(&self) -> Grid {
        let mut grid = Grid::new(self.size, self.size);
        for (index, &alive) in self.cells.iter().enumerate() {
            if alive {
                grid.cells[index] = 1;
            }
        }
        grid
    }

    /// Stamp the live cells into `cells` with its top-left corner at `(x0, y0)`.
    ///
    /// Cells outside the target grid are ignored, and dead draft cells leave the
    /// target untouched, so inserting never erases existing content.
    pub fn stamp_into(&self, cells: &mut [u8], width: usize, height: usize, x0: usize, y0: usize) {
        for y in 0..self.size {
            let ty = y0 + y;
            if ty >= height {
                break;
            }
            for x in 0..self.size {
                let tx = x0 + x;
                if tx >= width {
                    break;
                }
                if self.get(x, y) {
                    cells[ty * width + tx] = 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_with(points: &[(usize, usize)], size: usize) -> PatternDraft {
        let mut draft = PatternDraft::new(size);
        for &(x, y) in points {
            draft.set(x, y, true);
        }
        draft
    }

    #[test]
    fn rotating_four_times_is_a_round_trip() {
        let mut draft = draft_with(&[(1, 0), (2, 1), (0, 2)], 4);
        let original = draft.to_grid().cells;
        for _ in 0..4 {
            draft.rotate_cw();
        }
        assert_eq!(draft.to_grid().cells, original);
    }

    #[test]
    fn rotate_and_flip_move_cells_as_expected() {
        let mut draft = draft_with(&[(0, 0)], 3);
        draft.rotate_cw();
        assert!(draft.get(2, 0), "top-left rotates to top-right");

        let mut mirrored = draft_with(&[(0, 1)], 3);
        mirrored.flip_horizontal();
        assert!(mirrored.get(2, 1));

        let mut flipped = draft_with(&[(1, 0)], 3);
        flipped.flip_vertical();
        assert!(flipped.get(1, 2));
    }

    #[test]
    fn invert_and_clear_change_population() {
        let mut draft = draft_with(&[(0, 0), (1, 1)], 2);
        assert_eq!(draft.population(), 2);
        draft.invert();
        assert_eq!(draft.population(), 2);
        assert!(!draft.get(0, 0));
        draft.clear();
        assert!(draft.is_empty());
    }

    #[test]
    fn stamping_clips_to_the_target_grid() {
        let mut draft = draft_with(&[(0, 0), (1, 0), (0, 1)], 2);
        draft.set(3, 3, true); // outside a 2x2 draft is ignored
        let mut cells = vec![0u8; 3 * 3];
        draft.stamp_into(&mut cells, 3, 3, 2, 2);
        assert_eq!(cells[2 * 3 + 2], 1, "corner cell is stamped");
    }

    #[test]
    fn draft_converts_to_a_grid_for_export() {
        let draft = draft_with(&[(0, 0), (2, 1)], 3);
        let grid = draft.to_grid();
        assert_eq!(grid.population(), 2);
        assert_eq!(grid.get(1, 2), 1);
        let encoded = crate::rle::encode(&grid, "B3/S23");
        assert!(encoded.starts_with("x = "), "{encoded}");
    }
}
