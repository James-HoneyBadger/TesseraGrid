use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Immigration Game – multi-color Conway variant (states 1, 2, 3).
pub struct ImmigrationGame {
    grid: Grid,
    boundary: BoundaryMode,
}

impl ImmigrationGame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for ImmigrationGame {
    fn name(&self) -> &'static str {
        "Immigration Game"
    }

    fn step(&mut self) {
        let (counts, sums) = self.grid.neighbor_stats_alive(self.boundary);
        let old = self.grid.cells.clone();
        for i in 0..old.len() {
            let alive = old[i] > 0;
            let nc = counts[i];
            self.grid.cells[i] = if alive {
                if nc == 2 || nc == 3 {
                    old[i]
                } else {
                    0
                }
            } else if nc == 3 {
                let avg = sums[i] / 3;
                (avg % 3 + 1) as u8
            } else {
                0
            };
        }
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
        self.grid.set(y, x, (v + 1) % 4);
    }

    fn load_pattern(&mut self, pattern: &str) {
        self.grid.clear();
        let w = self.grid.width;
        let h = self.grid.height;
        let cx = (w / 2) as i32;
        let cy = (h / 2) as i32;

        match pattern {
            "Color Mix" => {
                for &(dx, dy) in &[(1i32, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
                    let x = cx - 20 + dx;
                    let y = cy - 15 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        self.grid.set(y as usize, x as usize, 1);
                    }
                }
                for &(dx, dy) in &[(0i32, 0), (1, 0), (2, 0)] {
                    let x = cx + dx - 1;
                    let y = cy - 15 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        self.grid.set(y as usize, x as usize, 2);
                    }
                }
            }
            _ => {
                let mut rng = rand::rng();
                for cell in self.grid.cells.iter_mut() {
                    if rng.random::<f32>() < 0.15 {
                        *cell = rng.random_range(1..=3);
                    }
                }
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Color Mix", "Random Soup"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
}
