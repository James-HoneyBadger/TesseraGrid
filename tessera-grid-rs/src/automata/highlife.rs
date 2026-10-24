use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// HighLife – B36/S23.
pub struct HighLife {
    grid: Grid,
    boundary: BoundaryMode,
}

impl HighLife {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for HighLife {
    fn name(&self) -> &'static str {
        "High Life"
    }

    fn step(&mut self) {
        let neighbors = self.grid.neighbor_counts(self.boundary, 1);
        self.grid.update_cells(|i, state| {
            let n = neighbors[i];
            if state == 1 {
                u8::from(n == 2 || n == 3)
            } else {
                u8::from(n == 3 || n == 6)
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
        let w = self.grid.width;
        let h = self.grid.height;
        let cx = w / 2;
        let cy = h / 2;
        match pattern {
            "Empty" => {}
            "Replicator" => {
                for &(dx, dy) in &[(1i32, 0), (0, 1), (1, 1), (2, 1), (0, 2), (2, 2), (1, 3)] {
                    let x = cx as i32 + dx - 1;
                    let y = cy as i32 + dy - 1;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        self.grid.set(y as usize, x as usize, 1);
                    }
                }
            }
            _ => {
                let mut rng = rand::rng();
                for cell in self.grid.cells.iter_mut() {
                    *cell = u8::from(rng.random::<f32>() < 0.15);
                }
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Replicator", "Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
    fn rule_string(&self) -> String {
        "B36/S23".to_owned()
    }
}
