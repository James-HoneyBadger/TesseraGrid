use rand::RngExt;

use crate::core::{BoundaryMode, Grid, NeighborBuffers};

use super::Automaton;

/// Generic life-like automaton with configurable B/S rule sets.
pub struct LifeLike {
    grid: Grid,
    boundary: BoundaryMode,
    birth: Vec<u8>,
    survival: Vec<u8>,
    /// Bit `n` set when the rule applies at `n` live neighbours.
    birth_mask: u16,
    survival_mask: u16,
    neighbors: NeighborBuffers,
}

/// Pack neighbour counts into a bitmask so the step function tests one bit
/// instead of scanning a `Vec` for every cell.
fn count_mask(counts: &[u8]) -> u16 {
    counts.iter().fold(0u16, |mask, &n| mask | 1 << n.min(8))
}

impl LifeLike {
    pub fn new(width: usize, height: usize, birth: Vec<u8>, survival: Vec<u8>) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            birth_mask: count_mask(&birth),
            survival_mask: count_mask(&survival),
            birth,
            survival,
            neighbors: NeighborBuffers::default(),
        }
    }

    /// Parse and validate a "B3/S23" style rule.
    ///
    /// A cell has at most eight neighbours, so digits above 8 cannot appear in
    /// a meaningful rule. Rejecting them matters: `B10/S1` used to parse as
    /// birth `[1, 0]`, and a birth rule containing 0 makes every dead cell with
    /// no live neighbours spring to life.
    pub fn try_parse_bs(rule: &str) -> Result<(Vec<u8>, Vec<u8>), String> {
        let upper = rule.to_uppercase();
        let mut birth = Vec::new();
        let mut survival = Vec::new();
        let mut in_birth = false;
        let mut in_survival = false;
        let mut seen = false;

        for ch in upper.chars() {
            match ch {
                'B' => {
                    in_birth = true;
                    in_survival = false;
                    seen = true;
                }
                'S' => {
                    in_birth = false;
                    in_survival = true;
                    seen = true;
                }
                '/' | ',' | ' ' => {}
                c if c.is_ascii_digit() => {
                    let digit = c as u8 - b'0';
                    if digit > 8 {
                        return Err(format!(
                            "neighbour count {digit} is out of range (0–8) in '{rule}'"
                        ));
                    }
                    if in_birth {
                        birth.push(digit);
                    } else if in_survival {
                        survival.push(digit);
                    } else {
                        return Err(format!("'{rule}' must use B…/S… notation"));
                    }
                }
                other => {
                    return Err(format!("unexpected '{other}' in rule '{rule}'"));
                }
            }
        }
        if !seen {
            return Err(format!("'{rule}' is not a B/S rule"));
        }
        Ok((birth, survival))
    }

    /// Render a rule back into B/S notation.
    pub fn format_bs(birth: &[u8], survival: &[u8]) -> String {
        let birth: String = birth.iter().map(|n| n.to_string()).collect();
        let survival: String = survival.iter().map(|n| n.to_string()).collect();
        format!("B{birth}/S{survival}")
    }
}

impl Automaton for LifeLike {
    fn name(&self) -> &'static str {
        "Custom Rules"
    }

    fn step(&mut self) {
        let (boundary, birth_mask, survival_mask) =
            (self.boundary, self.birth_mask, self.survival_mask);
        let Self {
            grid, neighbors, ..
        } = self;
        grid.neighbor_counts_into(boundary, 1, neighbors);
        let counts = &neighbors.counts;
        grid.update_cells(|i, state| {
            let n = u16::from(counts[i]);
            let rules = if state == 1 {
                survival_mask
            } else {
                birth_mask
            };
            u8::from((rules >> n) & 1 == 1)
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
        Self::format_bs(&self.birth, &self.survival)
    }

    fn set_rules(&mut self, birth: &[u8], survival: &[u8]) -> bool {
        self.birth.clear();
        self.birth.extend_from_slice(birth);
        self.survival.clear();
        self.survival.extend_from_slice(survival);
        self.birth_mask = count_mask(&self.birth);
        self.survival_mask = count_mask(&self.survival);
        true
    }
}
