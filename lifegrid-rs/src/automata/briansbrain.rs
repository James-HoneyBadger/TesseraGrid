use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Brian's Brain – three states: Off (0), Firing (1), Refractory (2).
pub struct BriansBrain {
    grid: Grid,
    boundary: BoundaryMode,
}

impl BriansBrain {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
        }
    }
}

impl Automaton for BriansBrain {
    fn name(&self) -> &'static str {
        "Brian's Brain"
    }

    fn step(&mut self) {
        let firing_neighbors = self.grid.neighbor_counts(self.boundary, 1);
        let old = self.grid.cells.clone();
        for i in 0..old.len() {
            self.grid.cells[i] = match old[i] {
                0 => u8::from(firing_neighbors[i] == 2), // Off → Firing if exactly 2 firing nbrs
                1 => 2,                                  // Firing → Refractory
                _ => 0,                                  // Refractory → Off
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

    fn load_pattern(&mut self, _pattern: &str) {
        self.grid.clear();
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            *cell = u8::from(rng.random::<f32>() < 0.08);
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
    fn rule_string(&self) -> String {
        "Brian'sBrain".to_owned()
    }

    fn max_state(&self) -> u8 {
        2
    }
}
