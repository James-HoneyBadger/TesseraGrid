//! Headless simulation controller shared by the GUI and the CLI.
//!
//! `Simulation` owns the automaton, generation counter, undo history, aging data
//! and frame recording. It has no dependency on egui, so it can be driven from
//! the command line or from tests.

use std::collections::VecDeque;

use crate::automata::{self, Automaton, LifeLike};
use crate::core::{BoundaryMode, Grid, Snapshot, UndoManager};

/// Number of population samples kept for the statistics panel.
pub const POP_HISTORY: usize = 500;
/// Default upper bound for buffered GIF frames.
pub const DEFAULT_MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;

/// Build an automaton for `mode`, falling back to Conway for unknown names.
pub fn make_automaton(
    mode: &str,
    width: usize,
    height: usize,
    boundary: BoundaryMode,
    birth: &[u8],
    survival: &[u8],
) -> Box<dyn Automaton> {
    let mut automaton: Box<dyn Automaton> = if mode == "Custom Rules" {
        Box::new(LifeLike::new(
            width,
            height,
            birth.to_vec(),
            survival.to_vec(),
        ))
    } else {
        automata::make_automaton(mode, width, height)
    };
    automaton.set_boundary(boundary);
    automaton
}

/// Boundary and B/S rules shared by the simulation constructors.
#[derive(Clone, Copy)]
pub struct RuleSpec<'a> {
    pub boundary: BoundaryMode,
    pub birth: &'a [u8],
    pub survival: &'a [u8],
}

impl<'a> RuleSpec<'a> {
    pub fn new(boundary: BoundaryMode, birth: &'a [u8], survival: &'a [u8]) -> Self {
        Self {
            boundary,
            birth,
            survival,
        }
    }
}

pub struct Simulation {
    automaton: Box<dyn Automaton>,
    /// Active mode name (used when rebuilding after a resize).
    mode: String,
    pub generation: u64,
    undo: UndoManager,
    age_grid: Vec<u32>,
    pop_history: VecDeque<u64>,
    pub record_frames: bool,
    frame_buffer: Vec<Vec<u8>>,
    frame_bytes: usize,
    pub max_frame_bytes: usize,
}

impl Simulation {
    pub fn new(
        mode: &str,
        width: usize,
        height: usize,
        rules: RuleSpec<'_>,
        pattern: &str,
    ) -> Self {
        let mut automaton = make_automaton(
            mode,
            width,
            height,
            rules.boundary,
            rules.birth,
            rules.survival,
        );
        automaton.load_pattern(pattern);
        let cells = automaton.get_grid().cells.len();
        Self {
            automaton,
            mode: mode.to_owned(),
            generation: 0,
            undo: UndoManager::new(100),
            age_grid: vec![0; cells],
            pop_history: VecDeque::with_capacity(POP_HISTORY),
            record_frames: false,
            frame_buffer: Vec::new(),
            frame_bytes: 0,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
        }
    }

    // ── Read access ──────────────────────────────────────────────────────────

    pub fn grid(&self) -> &Grid {
        self.automaton.get_grid()
    }

    pub fn width(&self) -> usize {
        self.grid().width
    }

    pub fn height(&self) -> usize {
        self.grid().height
    }

    pub fn population(&self) -> u64 {
        self.grid().population() as u64
    }

    pub fn density(&self) -> f32 {
        self.population() as f32 / (self.width() * self.height()).max(1) as f32
    }

    pub fn automaton_name(&self) -> &'static str {
        self.automaton.name()
    }

    pub fn rule_string(&self) -> String {
        self.automaton.rule_string()
    }

    pub fn boundary(&self) -> BoundaryMode {
        self.automaton.boundary()
    }

    pub fn max_state(&self) -> u8 {
        self.automaton.max_state()
    }

    pub fn supports_boundary(&self) -> bool {
        self.automaton.supports_boundary()
    }

    pub fn available_patterns(&self) -> &'static [&'static str] {
        self.automaton.available_patterns()
    }

    /// Snapshot of everything the canvas needs to render one frame.
    pub fn render_snapshot(&self) -> (usize, usize, Vec<u8>, Vec<u32>) {
        let grid = self.grid();
        (
            grid.width,
            grid.height,
            grid.cells.clone(),
            self.age_grid.clone(),
        )
    }

    pub fn pop_history(&self) -> &VecDeque<u64> {
        &self.pop_history
    }

    pub fn clear_pop_history(&mut self) {
        self.pop_history.clear();
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    pub fn frames(&self) -> &[Vec<u8>] {
        &self.frame_buffer
    }

    pub fn frame_count(&self) -> usize {
        self.frame_buffer.len()
    }

    pub fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    // ── Stepping ─────────────────────────────────────────────────────────────

    pub fn step(&mut self) {
        self.automaton.step();
        self.generation += 1;

        let cell_count = {
            let grid = self.automaton.get_grid();
            if self.age_grid.len() != grid.cells.len() {
                self.age_grid = vec![0; grid.cells.len()];
            }
            let age = &mut self.age_grid;
            let mut pop = 0u64;
            for (index, &state) in grid.cells.iter().enumerate() {
                if state != 0 {
                    age[index] = age[index].saturating_add(1);
                    pop += 1;
                } else {
                    age[index] = 0;
                }
            }
            self.pop_history.push_back(pop);
            if self.pop_history.len() > POP_HISTORY {
                self.pop_history.pop_front();
            }
            grid.cells.len()
        };

        if self.record_frames {
            let cells = self.automaton.get_grid().cells.clone();
            self.frame_bytes += cells.len();
            self.frame_buffer.push(cells);
            while self.frame_bytes > self.max_frame_bytes && self.frame_buffer.len() > 1 {
                if let Some(front) = self.frame_buffer.first() {
                    self.frame_bytes -= front.len();
                }
                self.frame_buffer.remove(0);
            }
        }
        let _ = cell_count;
    }

    /// Run up to `steps` generations, stopping early when `cancelled` is true.
    /// Returns the number of generations actually executed.
    pub fn run_steps(&mut self, steps: u64, mut cancelled: impl FnMut() -> bool) -> u64 {
        let mut done = 0;
        while done < steps {
            if cancelled() {
                break;
            }
            self.step();
            done += 1;
        }
        done
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    pub fn paint(&mut self, y: usize, x: usize, state: u8) {
        self.automaton.paint(y, x, state);
    }

    pub fn edit_cells(&mut self, mut edit: impl FnMut(&mut [u8])) {
        self.automaton.edit_cells(&mut edit);
    }

    /// Replace the cell buffer, keeping derived data consistent.
    pub fn set_cells(&mut self, cells: &[u8]) {
        self.automaton.set_cells(cells);
        if self.age_grid.len() != cells.len() {
            self.age_grid = vec![0; cells.len()];
        }
    }

    /// Fill the grid deterministically from `seed` and reset derived state.
    pub fn randomize(&mut self, seed: u64) {
        self.automaton.randomize(seed);
        self.generation = 0;
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }

    pub fn load_pattern(&mut self, pattern: &str) {
        self.automaton.load_pattern(pattern);
        self.age_grid = vec![0; self.grid().cells.len()];
    }

    pub fn set_boundary(&mut self, boundary: BoundaryMode) {
        self.automaton.set_boundary(boundary);
    }

    /// Clear the grid without changing the active mode or pattern list.
    pub fn reset(&mut self) {
        self.automaton.reset();
        self.generation = 0;
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }

    /// Rebuild the automaton for a new mode/pattern, resetting all derived state.
    pub fn rebuild(
        &mut self,
        mode: &str,
        width: usize,
        height: usize,
        rules: RuleSpec<'_>,
        pattern: &str,
    ) {
        self.automaton = make_automaton(
            mode,
            width,
            height,
            rules.boundary,
            rules.birth,
            rules.survival,
        );
        self.mode = mode.to_owned();
        self.automaton.load_pattern(pattern);
        self.generation = 0;
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
        self.undo.clear();
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }

    /// Resize the grid, optionally preserving the overlapping region.
    pub fn resize(&mut self, width: usize, height: usize, rules: RuleSpec<'_>, preserve: bool) {
        let old = if preserve {
            let grid = self.grid();
            Some((grid.width, grid.height, grid.cells.clone()))
        } else {
            None
        };
        let mut automaton = make_automaton(
            &self.mode,
            width,
            height,
            rules.boundary,
            rules.birth,
            rules.survival,
        );
        if let Some((old_w, old_h, cells)) = old {
            let mut resized = vec![0u8; width * height];
            let copy_w = old_w.min(width);
            let copy_h = old_h.min(height);
            for y in 0..copy_h {
                let src = y * old_w;
                let dst = y * width;
                resized[dst..dst + copy_w].copy_from_slice(&cells[src..src + copy_w]);
            }
            automaton.set_cells(&resized);
        }
        self.automaton = automaton;
        self.generation = 0;
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
        self.undo.clear();
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }

    // ── Undo / redo ──────────────────────────────────────────────────────────

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            cells: self.grid().cells.clone(),
            generation: self.generation,
        }
    }

    /// Record the current state so the next mutation can be undone.
    pub fn push_undo(&mut self) {
        let snapshot = self.snapshot();
        self.undo.push(snapshot);
    }

    pub fn undo(&mut self) -> bool {
        let current = self.snapshot();
        match self.undo.undo(current) {
            Some(previous) => {
                self.restore(previous);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        let current = self.snapshot();
        match self.undo.redo(current) {
            Some(next) => {
                self.restore(next);
                true
            }
            None => false,
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.automaton.set_cells(&snapshot.cells);
        self.generation = snapshot.generation;
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
    }

    // ── Frame recording ──────────────────────────────────────────────────────

    pub fn clear_frames(&mut self) {
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }
}
