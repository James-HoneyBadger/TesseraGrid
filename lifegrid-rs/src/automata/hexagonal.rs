use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Hexagonal Game of Life – B2/S34 on an offset-coordinate grid.
///
/// Uses "Odd-r" horizontal layout for neighbour calculation.
pub struct HexagonalLife {
    grid: Grid,
}

impl HexagonalLife {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
        }
    }

    fn hex_neighbors(&self, y: usize, x: usize) -> u8 {
        let w = self.grid.width;
        let h = self.grid.height;
        let wi = w as i32;
        let hi = h as i32;

        // Six hex neighbours for odd-r offset layout.
        // Even rows: left, right, up-left, up, down-left, down.
        // Odd rows:  left, right, up, up-right, down, down-right.
        const OFFSETS_EVEN: &[(i32, i32)] = &[(-1, 0), (1, 0), (-1, -1), (0, -1), (-1, 1), (0, 1)];
        const OFFSETS_ODD: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (1, -1), (0, 1), (1, 1)];
        let offsets = if y.is_multiple_of(2) {
            OFFSETS_EVEN
        } else {
            OFFSETS_ODD
        };

        let mut count = 0u8;
        for &(dx, dy) in offsets {
            let nx = (x as i32 + dx).rem_euclid(wi) as usize;
            let ny = (y as i32 + dy).rem_euclid(hi) as usize;
            if self.grid.get(ny, nx) > 0 {
                count += 1;
            }
        }
        count
    }
}

impl Automaton for HexagonalLife {
    fn name(&self) -> &'static str {
        "Hexagonal Life"
    }

    fn step(&mut self) {
        let h = self.grid.height;
        let w = self.grid.width;
        let mut next = vec![0u8; h * w];
        for y in 0..h {
            for x in 0..w {
                let alive = self.grid.get(y, x) > 0;
                let n = self.hex_neighbors(y, x);
                let i = y * w + x;
                next[i] = if alive {
                    u8::from(n == 3 || n == 4)
                } else {
                    u8::from(n == 2)
                };
            }
        }
        self.grid.cells = next;
    }

    fn reset(&mut self) {
        self.grid.clear();
    }

    fn get_grid(&self) -> &Grid {
        &self.grid
    }

    fn get_grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    fn handle_click(&mut self, x: usize, y: usize) {
        let v = self.grid.get(y, x);
        self.grid.set(y, x, 1 - v);
    }

    fn load_pattern(&mut self, _pattern: &str) {
        self.grid.clear();
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            *cell = u8::from(rng.random::<f32>() < 0.15);
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Random Soup"]
    }

    fn boundary(&self) -> BoundaryMode {
        BoundaryMode::Wrap
    }

    fn set_boundary(&mut self, _mode: BoundaryMode) {}
}
