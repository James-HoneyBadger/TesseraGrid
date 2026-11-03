//! Elementary (one-dimensional, two-state, nearest-neighbour) automata.
//!
//! These are Stephen Wolfram's 256 "elementary rules". The grid is used as a
//! space-time diagram: each row is one generation, the newest row sits below the
//! previous one, and once the diagram is full it scrolls upwards.
//!
//! Rule 110 is Turing complete (Cook, 2004), which makes this mode the smallest
//! self-contained demonstration of universal computation in the simulator.

use rand::RngExt;

use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Rule used unless the caller supplies another one.
///
/// Rule 110 is the classic Turing-complete elementary rule.
pub const DEFAULT_RULE: u8 = 110;

/// Elementary cellular automaton driven by a Wolfram code.
pub struct Elementary1D {
    grid: Grid,
    boundary: BoundaryMode,
    /// Wolfram code, 0–255: bit `n` is the next state for neighbourhood `n`.
    rule: u8,
    /// Lookup for `(left << 2) | (centre << 1) | right`.
    table: [bool; 8],
    /// Row holding the current generation.
    row: usize,
    /// Scratch row, reused between steps.
    scratch: Vec<u8>,
}

impl Elementary1D {
    pub fn new(width: usize, height: usize, rule: u8) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            rule,
            table: rule_table(rule),
            row: 0,
            scratch: Vec::new(),
        }
    }

    /// The active Wolfram code.
    pub fn rule(&self) -> u8 {
        self.rule
    }

    /// Replace the rule without touching the diagram.
    pub fn set_rule(&mut self, rule: u8) {
        self.rule = rule;
        self.table = rule_table(rule);
    }

    /// Compute the generation after `row` into `self.scratch`.
    fn compute_row(&mut self, row: usize) {
        let w = self.grid.width;
        let boundary = self.boundary;
        let Self {
            grid,
            scratch,
            table,
            ..
        } = self;
        let cells = &grid.cells[row * w..row * w + w];
        scratch.clear();
        scratch.reserve(w);
        let live = |index: Option<usize>| u8::from(index.is_some_and(|i| cells[i] > 0));
        for x in 0..w {
            let neighbourhood = (live(boundary.resolve_axis(x as i32 - 1, w)) << 2)
                | (live(Some(x)) << 1)
                | live(boundary.resolve_axis(x as i32 + 1, w));
            scratch.push(u8::from(table[usize::from(neighbourhood)]));
        }
    }

    /// Fill `row` with a reproducible random configuration.
    fn seed_random_row(&mut self, row: usize, seed: u64) {
        use rand::SeedableRng;
        let w = self.grid.width;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let base = row * w;
        for x in 0..w {
            self.grid.cells[base + x] = u8::from(rng.random::<f32>() < 0.5);
        }
    }
}

fn rule_table(rule: u8) -> [bool; 8] {
    let mut table = [false; 8];
    for (index, entry) in table.iter_mut().enumerate() {
        *entry = rule & (1 << index) != 0;
    }
    table
}

impl Automaton for Elementary1D {
    fn name(&self) -> &'static str {
        "Elementary 1D"
    }

    fn step(&mut self) {
        let (w, h) = (self.grid.width, self.grid.height);
        if w == 0 || h == 0 {
            return;
        }
        // Append below the current generation; once the diagram is full, scroll
        // it up so the newest row is always the last one.
        let target = if self.row + 1 < h {
            self.row + 1
        } else {
            self.grid.cells.copy_within(w.., 0);
            h - 1
        };
        let source = target.saturating_sub(1);
        self.compute_row(source);
        let base = target * w;
        self.grid.cells[base..base + w].copy_from_slice(&self.scratch);
        self.row = target;
    }

    fn reset(&mut self) {
        self.grid.clear();
        self.row = 0;
    }

    fn get_grid(&self) -> &Grid {
        &self.grid
    }

    fn get_grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
    }

    fn load_pattern(&mut self, pattern: &str) {
        self.grid.clear();
        self.row = 0;
        let w = self.grid.width;
        match pattern {
            "Empty" => {}
            "Random Row" => self.seed_random_row(0, 0x5eed_1234),
            // "Single Cell" and any unknown name: the classic seed.
            _ => {
                if w > 0 {
                    self.grid.set(0, w / 2, 1);
                }
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Single Cell", "Random Row", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }

    fn rule_string(&self) -> String {
        format!("W{}", self.rule)
    }

    fn set_rules(&mut self, birth: &[u8], _survival: &[u8]) -> bool {
        // The sidebar's B/S parser is not used for this mode; a single value is
        // accepted so scripted callers can still set the rule this way.
        if let [rule] = birth {
            self.set_rule(*rule);
            return true;
        }
        false
    }

    fn randomize(&mut self, seed: u64) {
        self.grid.clear();
        self.row = 0;
        self.seed_random_row(0, seed);
    }

    /// Only the newest row is simulation state; the rest is history.
    fn active_rows(&self) -> (usize, usize) {
        (self.row, (self.row + 1).min(self.grid.height))
    }

    fn aux_state(&self) -> Vec<u8> {
        (self.row as u64).to_le_bytes().to_vec()
    }

    fn set_aux_state(&mut self, state: &[u8]) {
        if state.len() != 8 {
            return;
        }
        let row = u64::from_le_bytes(state.try_into().unwrap_or_default()) as usize;
        self.row = row.min(self.grid.height.saturating_sub(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference step for a single row, resolving neighbours by hand.
    fn reference_row(cells: &[u8], rule: u8, boundary: BoundaryMode) -> Vec<u8> {
        let w = cells.len();
        (0..w)
            .map(|x| {
                let get = |i: Option<usize>| u8::from(i.is_some_and(|i| cells[i] > 0));
                let left = get(boundary.resolve_axis(x as i32 - 1, w));
                let centre = u8::from(cells[x] > 0);
                let right = get(boundary.resolve_axis(x as i32 + 1, w));
                let index = (left << 2) | (centre << 1) | right;
                u8::from(rule & (1 << index) != 0)
            })
            .collect()
    }

    fn seeded(width: usize, height: usize, rule: u8) -> Elementary1D {
        let mut auto = Elementary1D::new(width, height, rule);
        auto.load_pattern("Single Cell");
        auto
    }

    #[test]
    fn rule_30_grows_the_classic_three_cell_row() {
        let mut auto = seeded(21, 8, 30);
        auto.step();
        let row = &auto.grid.cells[auto.grid.width..2 * auto.grid.width];
        let live: Vec<usize> = row
            .iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(live, vec![9, 10, 11], "row 1 of rule 30");
    }

    #[test]
    fn every_rule_matches_the_reference_row() {
        for rule in [0u8, 18, 22, 30, 45, 90, 110, 150, 184, 255] {
            for boundary in BoundaryMode::all().iter().copied() {
                let mut auto = seeded(17, 4, rule);
                auto.set_boundary(boundary);
                let first = auto.grid.cells[..17].to_vec();
                auto.step();
                assert_eq!(
                    &auto.grid.cells[17..34],
                    reference_row(&first, rule, boundary).as_slice(),
                    "rule {rule}, boundary {boundary:?}"
                );
            }
        }
    }

    #[test]
    fn the_diagram_fills_then_scrolls() {
        let mut auto = seeded(11, 4, 110);
        for _ in 0..3 {
            auto.step();
        }
        assert_eq!(auto.row, 3, "rows fill from the top");
        let before = auto.grid.cells.clone();
        auto.step();
        assert_eq!(auto.row, 3, "the newest row stays at the bottom");
        assert_ne!(auto.grid.cells, before);
        assert_eq!(
            &auto.grid.cells[..2 * 11],
            &before[11..3 * 11],
            "history scrolls up by one row"
        );
    }

    #[test]
    fn a_one_row_diagram_steps_in_place() {
        let mut auto = seeded(9, 1, 30);
        auto.step();
        assert_eq!(auto.row, 0);
        assert_eq!(
            auto.grid.cells,
            reference_row(&[0, 0, 0, 0, 1, 0, 0, 0, 0], 30, BoundaryMode::Wrap)
        );
    }

    #[test]
    fn only_the_newest_row_counts_as_population() {
        let mut auto = seeded(21, 8, 30);
        for _ in 0..5 {
            auto.step();
        }
        let (start, end) = auto.active_rows();
        assert_eq!((start, end), (5, 6));
        let active: usize = auto.grid.cells[start * 21..end * 21]
            .iter()
            .filter(|&&cell| cell > 0)
            .count();
        assert!(active > 0);
        assert!(auto.grid.population() > active, "history is larger");
    }

    #[test]
    fn position_survives_an_aux_round_trip() {
        let mut auto = seeded(9, 6, 110);
        for _ in 0..4 {
            auto.step();
        }
        let aux = auto.aux_state();
        let cells = auto.grid.cells.clone();

        // What the next generation must look like.
        auto.step();
        let expected = auto.grid.cells.clone();

        // Rewind to the captured state and replay.
        auto.reset();
        auto.set_cells(&cells);
        auto.set_aux_state(&aux);
        assert_eq!(auto.row, 4);
        auto.step();
        assert_eq!(auto.grid.cells, expected, "replay must land on row 5");
    }

    #[test]
    fn randomize_is_deterministic_and_keeps_the_rule() {
        let mut a = Elementary1D::new(31, 4, 30);
        let mut b = Elementary1D::new(31, 4, 30);
        a.randomize(7);
        b.randomize(7);
        assert_eq!(a.grid.cells, b.grid.cells);
        assert_eq!(a.row, 0);
        assert_eq!(a.rule_string(), "W30");
    }
}
