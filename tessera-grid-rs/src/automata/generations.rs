use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Generations – life-like birth/survival with N fading states.
///
/// State 1 is alive; states 2..=n_states-1 are decaying; state 0 is dead.
pub struct Generations {
    grid: Grid,
    boundary: BoundaryMode,
    birth: Vec<u8>,
    survival: Vec<u8>,
    n_states: u8,
}

impl Generations {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            birth: vec![3],
            survival: vec![2, 3],
            n_states: 8,
        }
    }
}

impl Automaton for Generations {
    fn name(&self) -> &'static str {
        "Generations"
    }

    fn step(&mut self) {
        // Count only fully-alive (state == 1) neighbours
        let neighbors = self.grid.neighbor_counts(self.boundary, 1);
        let old = self.grid.cells.clone();
        let n_states = self.n_states;
        for i in 0..old.len() {
            let s = old[i];
            let nc = neighbors[i];
            self.grid.cells[i] = match s {
                0 => u8::from(self.birth.contains(&nc)),
                1 => {
                    if self.survival.contains(&nc) {
                        1
                    } else {
                        2 // begin decay
                    }
                }
                v if v < n_states - 1 => v + 1,
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

    fn load_pattern(&mut self, pattern: &str) {
        self.grid.clear();
        if pattern == "Empty" {
            return;
        }
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            *cell = u8::from(rng.random::<f32>() < 0.15);
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
    fn rule_string(&self) -> String {
        let birth: String = self.birth.iter().map(|n| n.to_string()).collect();
        let survival: String = self.survival.iter().map(|n| n.to_string()).collect();
        format!("B{birth}/S{survival}/C{}", self.n_states)
    }

    fn max_state(&self) -> u8 {
        self.n_states - 1
    }
}
