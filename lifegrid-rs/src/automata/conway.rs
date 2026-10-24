use rand::RngExt;

use crate::core::{BoundaryMode, Grid};
use crate::patterns;

use super::Automaton;

pub struct ConwayLife {
    grid: Grid,
    boundary: BoundaryMode,
}

impl ConwayLife {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for ConwayLife {
    fn name(&self) -> &'static str {
        "Conway's Game of Life"
    }

    fn step(&mut self) {
        let neighbors = self.grid.neighbor_counts(self.boundary, 1);
        let old = self.grid.cells.clone();
        for i in 0..old.len() {
            let alive = old[i] == 1;
            let n = neighbors[i];
            self.grid.cells[i] = if alive {
                u8::from(n == 2 || n == 3)
            } else {
                u8::from(n == 3)
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
        self.grid.set(y, x, 1 - v);
    }

    fn load_pattern(&mut self, pattern: &str) {
        self.grid.clear();
        let w = self.grid.width;
        let h = self.grid.height;
        if pattern == "Random Soup" {
            let mut rng = rand::rng();
            for cell in self.grid.cells.iter_mut() {
                *cell = u8::from(rng.random::<f32>() < 0.15);
            }
            return;
        }
        if let Some(pts) = patterns::conway_pattern_points(pattern) {
            let cx = (w / 2) as i32;
            let cy = (h / 2) as i32;
            for &(dx, dy) in pts {
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && x < w as i32 && y >= 0 && y < h as i32 {
                    self.grid.set(y as usize, x as usize, 1);
                }
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        patterns::CONWAY_PATTERN_NAMES
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
}
