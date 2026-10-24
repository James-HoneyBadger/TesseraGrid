use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Rainbow Game – six-colour cellular automaton (states 1–6).
pub struct RainbowGame {
    grid: Grid,
    boundary: BoundaryMode,
}

impl RainbowGame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for RainbowGame {
    fn name(&self) -> &'static str {
        "Rainbow Game"
    }

    fn step(&mut self) {
        let (counts, sums) = self.grid.neighbor_stats_alive(self.boundary);
        self.grid.update_cells(|i, state| {
            let nc = counts[i];
            if state > 0 {
                if nc == 2 || nc == 3 {
                    state
                } else {
                    0
                }
            } else if nc == 3 {
                (sums[i] / 3).clamp(1, 6) as u8
            } else {
                0
            }
        });
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

    fn load_pattern(&mut self, pattern: &str) {
        self.grid.clear();
        if pattern == "Empty" {
            return;
        }
        let mut rng = rand::rng();
        if pattern == "Rainbow Mix" {
            // Concentric colour bands give the cyclic rule something to work with.
            let w = self.grid.width;
            let h = self.grid.height;
            for y in 0..h {
                for x in 0..w {
                    let dx = x as i32 - w as i32 / 2;
                    let dy = y as i32 - h as i32 / 2;
                    let ring = ((dx.abs() + dy.abs()) / 4) % 6 + 1;
                    self.grid.set(y, x, ring as u8);
                }
            }
            return;
        }
        for cell in self.grid.cells.iter_mut() {
            if rng.random::<f32>() < 0.15 {
                *cell = rng.random_range(1..=6);
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Rainbow Mix", "Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
    fn max_state(&self) -> u8 {
        6
    }
}
