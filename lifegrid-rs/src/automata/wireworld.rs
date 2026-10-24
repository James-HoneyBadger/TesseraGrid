use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Wireworld – four states: Empty (0), Head (1), Tail (2), Conductor (3).
pub struct Wireworld {
    grid: Grid,
    boundary: BoundaryMode,
}

impl Wireworld {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for Wireworld {
    fn name(&self) -> &'static str {
        "Wireworld"
    }

    fn step(&mut self) {
        let head_neighbors = self.grid.neighbor_counts(self.boundary, 1);
        let old = self.grid.cells.clone();
        for i in 0..old.len() {
            self.grid.cells[i] = match old[i] {
                0 => 0,
                1 => 2,
                2 => 3,
                3 => {
                    let hn = head_neighbors[i];
                    if hn == 1 || hn == 2 {
                        1
                    } else {
                        3
                    }
                }
                _ => 0,
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

    fn load_pattern(&mut self, _pattern: &str) {
        self.grid.clear();
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            let r: f32 = rng.random();
            *cell = if r < 0.02 {
                1
            } else if r < 0.12 {
                3
            } else {
                0
            };
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Random Soup"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
}
