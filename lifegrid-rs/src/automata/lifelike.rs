use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Generic life-like automaton with configurable B/S rule sets.
pub struct LifeLike {
    grid: Grid,
    boundary: BoundaryMode,
    birth: Vec<u8>,
    survival: Vec<u8>,
}

impl LifeLike {
    pub fn new(width: usize, height: usize, birth: Vec<u8>, survival: Vec<u8>) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            birth,
            survival,
        }
    }

    #[allow(dead_code)]
    pub fn set_rules(&mut self, birth: Vec<u8>, survival: Vec<u8>) {
        self.birth = birth;
        self.survival = survival;
    }

    /// Parse a "B3/S23" style string into (birth, survival).
    pub fn parse_bs(rule: &str) -> (Vec<u8>, Vec<u8>) {
        let rule = rule.to_uppercase();
        let mut birth = Vec::new();
        let mut survival = Vec::new();
        let mut in_birth = false;
        let mut in_survival = false;
        for ch in rule.chars() {
            match ch {
                'B' => {
                    in_birth = true;
                    in_survival = false;
                }
                'S' => {
                    in_birth = false;
                    in_survival = true;
                }
                '/' => {}
                c if c.is_ascii_digit() => {
                    let d = c as u8 - b'0';
                    if in_birth {
                        birth.push(d);
                    } else if in_survival {
                        survival.push(d);
                    }
                }
                _ => {}
            }
        }
        (birth, survival)
    }
}

impl Automaton for LifeLike {
    fn name(&self) -> &'static str {
        "Custom Rules"
    }

    fn step(&mut self) {
        let neighbors = self.grid.neighbor_counts(self.boundary, 1);
        let old = self.grid.cells.clone();
        for i in 0..old.len() {
            let alive = old[i] == 1;
            let n = neighbors[i];
            self.grid.cells[i] = if alive {
                u8::from(self.survival.contains(&n))
            } else {
                u8::from(self.birth.contains(&n))
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
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
}
