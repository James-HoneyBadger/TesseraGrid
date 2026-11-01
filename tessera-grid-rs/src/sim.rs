//! Headless simulation controller shared by the GUI and the CLI.
//!
//! `Simulation` owns the automaton, generation counter, undo history, aging data
//! and frame recording. It has no dependency on egui, so it can be driven from
//! the command line or from tests.

use std::collections::VecDeque;

use crate::automata::{self, Automaton, LifeLike, Mode};
use crate::core::{BoundaryMode, Grid, Snapshot, UndoManager};

/// Number of population samples kept for the statistics panel.
pub const POP_HISTORY: usize = 500;
/// Default upper bound for buffered GIF frames.
pub const DEFAULT_MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;
/// Maximum number of generations kept for the timeline scrubber.
pub const TIMELINE_MAX_ENTRIES: usize = 512;
/// Default byte budget for timeline snapshots.
pub const DEFAULT_MAX_TIMELINE_BYTES: usize = 64 * 1024 * 1024;

/// Largest grid side accepted from any source (config, CLI, state file).
///
/// Clamping here protects every constructor, including the ones fed by files
/// and clipboard text, from multi-gigabyte allocations.
pub const MAX_GRID_SIDE: usize = 4096;

/// Build an automaton for `mode`, falling back to Conway for unknown names.
pub fn make_automaton(
    mode: &str,
    width: usize,
    height: usize,
    boundary: BoundaryMode,
    birth: &[u8],
    survival: &[u8],
) -> Box<dyn Automaton> {
    let width = width.clamp(1, MAX_GRID_SIDE);
    let height = height.clamp(1, MAX_GRID_SIDE);
    let uses_custom_rules = Mode::from_name(mode).is_some_and(Mode::uses_custom_rules);
    let mut automaton: Box<dyn Automaton> = if uses_custom_rules {
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
    frame_buffer: VecDeque<Vec<u8>>,
    frame_bytes: usize,
    pub max_frame_bytes: usize,
    /// Recent generations for the timeline scrubber, oldest first.
    timeline: VecDeque<Snapshot>,
    timeline_bytes: usize,
    /// `Some(index)` while the view shows an older generation instead of the
    /// newest one; `None` means "live".
    timeline_pos: Option<usize>,
    pub max_timeline_bytes: usize,
    /// Bumped on every observable grid change, so derived data can be cached
    /// between frames instead of recomputed.
    revision: u64,
    /// Live-cell count kept in step with the grid, so the status bar does not
    /// rescan the grid every frame.
    population: u64,
    /// Reusable render buffers, refilled only when the revision changes.
    render_cache: Option<RenderCache>,
}

/// Grid data handed to the canvas for one frame.
///
/// The buffers are owned by the cache and handed back after rendering, so a
/// steady frame costs no allocation or grid copy.
pub struct RenderFrame {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<u8>,
    pub ages: Vec<u32>,
    revision: u64,
}

struct RenderCache {
    revision: u64,
    cells: Vec<u8>,
    ages: Vec<u32>,
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
        let population = automaton.get_grid().population() as u64;
        Self {
            automaton,
            mode: mode.to_owned(),
            generation: 0,
            undo: UndoManager::new(100),
            age_grid: vec![0; cells],
            pop_history: VecDeque::with_capacity(POP_HISTORY),
            record_frames: false,
            frame_buffer: VecDeque::new(),
            frame_bytes: 0,
            max_frame_bytes: DEFAULT_MAX_FRAME_BYTES,
            timeline: VecDeque::new(),
            timeline_bytes: 0,
            timeline_pos: None,
            max_timeline_bytes: DEFAULT_MAX_TIMELINE_BYTES,
            revision: 0,
            population,
            render_cache: None,
        }
    }

    /// Monotonic counter bumped whenever the grid changes.
    ///
    /// Views use it to reuse derived data (population, render buffers) instead
    /// of recomputing or copying the grid every frame.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn bump(&mut self) {
        self.revision += 1;
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
        self.population
    }

    /// Recompute the live-cell count from the grid.
    fn recount_population(&mut self) {
        self.population = self.grid().population() as u64;
    }

    pub fn density(&self) -> f32 {
        self.population() as f32 / (self.width() * self.height()).max(1) as f32
    }

    pub fn mode(&self) -> &str {
        &self.mode
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

    /// Borrow the grid data the canvas needs for one frame.
    ///
    /// The buffers are moved out of the cache so the caller can keep mutating
    /// `self` while rendering; hand them back with
    /// [`Simulation::return_render_frame`]. The grid is only copied when it has
    /// actually changed since the last frame.
    pub fn take_render_frame(&mut self) -> RenderFrame {
        let mut cache = self.render_cache.take().unwrap_or(RenderCache {
            revision: u64::MAX,
            cells: Vec::new(),
            ages: Vec::new(),
        });
        let grid = self.grid();
        let (width, height) = (grid.width, grid.height);
        if cache.revision != self.revision
            || cache.cells.len() != grid.cells.len()
            || cache.ages.len() != grid.cells.len()
        {
            cache.cells.clear();
            cache.cells.extend_from_slice(&grid.cells);
            cache.ages.clear();
            cache.ages.extend_from_slice(&self.age_grid);
            cache.revision = self.revision;
        }
        RenderFrame {
            width,
            height,
            cells: std::mem::take(&mut cache.cells),
            ages: std::mem::take(&mut cache.ages),
            revision: cache.revision,
        }
    }

    /// Return buffers previously obtained from [`Simulation::take_render_frame`].
    pub fn return_render_frame(&mut self, mut frame: RenderFrame) {
        frame.cells.clear();
        frame.ages.clear();
        self.render_cache = Some(RenderCache {
            revision: frame.revision,
            cells: frame.cells,
            ages: frame.ages,
        });
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

    /// Recorded frames in play order.
    ///
    /// `&mut self` is required because a `VecDeque` may be stored in two
    /// disjoint slices; this makes it contiguous so the result is one slice.
    pub fn frames(&mut self) -> &[Vec<u8>] {
        self.frame_buffer.make_contiguous()
    }

    pub fn frame_count(&self) -> usize {
        self.frame_buffer.len()
    }

    pub fn frame_bytes(&self) -> usize {
        self.frame_bytes
    }

    // ── Stepping ─────────────────────────────────────────────────────────────

    pub fn step(&mut self) {
        self.rewind_to_live();
        if self.timeline.is_empty() {
            // Seed with the pre-step state so the scrubber can reach generation 0.
            self.record_timeline();
        }
        self.automaton.step();
        self.generation += 1;

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
        self.population = pop;
        self.pop_history.push_back(pop);
        if self.pop_history.len() > POP_HISTORY {
            self.pop_history.pop_front();
        }

        if self.record_frames {
            let cells = self.automaton.get_grid().cells.clone();
            self.frame_bytes += cells.len();
            self.frame_buffer.push_back(cells);
            while self.frame_bytes > self.max_frame_bytes && self.frame_buffer.len() > 1 {
                if let Some(front) = self.frame_buffer.pop_front() {
                    self.frame_bytes -= front.len();
                }
            }
        }
        self.bump();
        self.record_timeline();
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
        let grid = self.automaton.get_grid();
        if y >= grid.height || x >= grid.width {
            return;
        }
        let before = grid.get(y, x);
        self.automaton.paint(y, x, state);
        let after = self.automaton.get_grid().get(y, x);
        if before == after {
            return;
        }
        self.population -= u64::from(before != 0);
        self.population += u64::from(after != 0);
        self.bump();
    }

    pub fn edit_cells(&mut self, mut edit: impl FnMut(&mut [u8])) {
        self.automaton.edit_cells(&mut edit);
        self.recount_population();
        self.bump();
    }

    /// Replace the cell buffer, keeping derived data consistent.
    pub fn set_cells(&mut self, cells: &[u8]) {
        self.automaton.set_cells(cells);
        if self.age_grid.len() != cells.len() {
            self.age_grid = vec![0; cells.len()];
        }
        self.recount_population();
        self.bump();
    }

    /// Drop everything derived from the grid after a wholesale change.
    ///
    /// Every path that replaces the whole grid goes through here so no derived
    /// buffer (ages, chart, frames, timeline, population) can be forgotten.
    fn reset_derived_state(&mut self) {
        self.age_grid = vec![0; self.grid().cells.len()];
        self.pop_history.clear();
        self.frame_buffer.clear();
        self.frame_bytes = 0;
        self.clear_timeline();
        self.recount_population();
        self.bump();
    }

    /// Fill the grid deterministically from `seed` and reset derived state.
    pub fn randomize(&mut self, seed: u64) {
        self.automaton.randomize(seed);
        self.generation = 0;
        self.reset_derived_state();
    }

    pub fn load_pattern(&mut self, pattern: &str) {
        self.automaton.load_pattern(pattern);
        self.reset_derived_state();
    }

    pub fn set_boundary(&mut self, boundary: BoundaryMode) {
        self.automaton.set_boundary(boundary);
    }

    /// Clear the grid without changing the active mode or pattern list.
    pub fn reset(&mut self) {
        self.automaton.reset();
        self.generation = 0;
        self.reset_derived_state();
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
        self.undo.clear();
        self.reset_derived_state();
    }

    /// Resize the grid, optionally preserving the overlapping region.
    pub fn resize(&mut self, width: usize, height: usize, rules: RuleSpec<'_>, preserve: bool) {
        let width = width.clamp(1, MAX_GRID_SIDE);
        let height = height.clamp(1, MAX_GRID_SIDE);
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
        self.undo.clear();
        self.reset_derived_state();
    }

    // ── Undo / redo ──────────────────────────────────────────────────────────

    fn snapshot(&self) -> Snapshot {
        Snapshot::new(self.automaton.state_cells(), self.generation)
            .with_aux(self.automaton.aux_state())
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
        self.apply_snapshot(&snapshot);
        self.clear_timeline();
    }

    /// Put the grid and automaton back into a recorded state.
    fn apply_snapshot(&mut self, snapshot: &Snapshot) {
        self.automaton.set_cells(&snapshot.cells);
        self.automaton.set_aux_state(&snapshot.aux);
        self.generation = snapshot.generation;
        self.age_grid = vec![0; snapshot.cells.len()];
        self.pop_history.clear();
        self.recount_population();
        self.bump();
    }

    // ── Timeline ─────────────────────────────────────────────────────────────

    /// Number of generations currently held by the timeline (including the
    /// newest state), or zero when nothing has been recorded yet.
    pub fn timeline_len(&self) -> usize {
        self.timeline.len()
    }

    /// Index of the generation being displayed, or `None` while live.
    pub fn timeline_pos(&self) -> Option<usize> {
        self.timeline_pos
    }

    /// Generation number stored at `index`, if it exists.
    pub fn timeline_generation(&self, index: usize) -> Option<u64> {
        self.timeline.get(index).map(|entry| entry.generation)
    }

    /// Show an older generation without advancing the simulation.
    pub fn scrub(&mut self, index: usize) -> bool {
        let Some(snapshot) = self.timeline.get(index).cloned() else {
            return false;
        };
        self.apply_snapshot(&snapshot);
        // The newest entry *is* the live state, so scrubbing to it leaves the
        // timeline live rather than pinned.
        self.timeline_pos = (index + 1 < self.timeline.len()).then_some(index);
        true
    }

    /// Jump back to the newest generation.
    pub fn resume_live(&mut self) -> bool {
        match self.timeline.len() {
            0 => false,
            len => self.scrub(len - 1),
        }
    }

    /// Drop generations after the scrub position and continue live from there.
    ///
    /// Called before every step so playing from a scrubbed position forks the
    /// history instead of appending to a future that is no longer reachable.
    fn rewind_to_live(&mut self) {
        if let Some(position) = self.timeline_pos.take() {
            while self.timeline.len() > position + 1 {
                if let Some(entry) = self.timeline.pop_back() {
                    self.timeline_bytes -= entry.byte_len();
                }
            }
        }
    }

    fn record_timeline(&mut self) {
        let snapshot = self.snapshot();
        self.timeline_bytes += snapshot.byte_len();
        self.timeline.push_back(snapshot);
        while self.timeline.len() > TIMELINE_MAX_ENTRIES
            || (self.timeline_bytes > self.max_timeline_bytes && self.timeline.len() > 1)
        {
            let Some(entry) = self.timeline.pop_front() else {
                break;
            };
            self.timeline_bytes -= entry.byte_len();
        }
        self.timeline_pos = None;
    }

    fn clear_timeline(&mut self) {
        self.timeline.clear();
        self.timeline_bytes = 0;
        self.timeline_pos = None;
    }

    // ── Frame recording ──────────────────────────────────────────────────────

    pub fn clear_frames(&mut self) {
        self.frame_buffer.clear();
        self.frame_bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sim(mode: &str) -> Simulation {
        Simulation::new(
            mode,
            20,
            20,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
            "Empty",
        )
    }

    #[test]
    fn randomize_is_deterministic() {
        let mut a = sim("Conway's Game of Life");
        let mut b = sim("Conway's Game of Life");
        a.randomize(7);
        b.randomize(7);
        assert_eq!(a.grid().cells, b.grid().cells);
        b.randomize(8);
        assert_ne!(a.grid().cells, b.grid().cells);
    }

    #[test]
    fn undo_restores_cells_and_generation() {
        let mut s = sim("Conway's Game of Life");
        s.paint(1, 1, 1);
        s.push_undo();
        s.paint(2, 2, 1);
        s.step();
        let after_step = s.grid().cells.clone();
        assert_eq!(s.generation, 1);

        assert!(s.undo());
        assert_eq!(s.generation, 0);
        assert_eq!(s.grid().get(2, 2), 0);

        assert!(s.redo());
        assert_eq!(s.grid().cells, after_step);
        assert_eq!(s.generation, 1);
    }

    #[test]
    fn resize_preserves_overlapping_cells() {
        let mut s = sim("Conway's Game of Life");
        s.paint(1, 1, 1);
        s.resize(
            30,
            30,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
            true,
        );
        assert_eq!(s.grid().width, 30);
        assert_eq!(s.grid().height, 30);
        assert_eq!(s.grid().get(1, 1), 1);
        assert_eq!(s.generation, 0);
    }

    #[test]
    fn run_steps_can_be_cancelled() {
        let mut s = sim("Conway's Game of Life");
        assert_eq!(s.run_steps(100, || true), 0);
        assert_eq!(s.generation, 0);
        assert_eq!(s.run_steps(5, || false), 5);
        assert_eq!(s.generation, 5);
    }

    #[test]
    fn timeline_records_generations_and_scrubs_back() {
        let mut s = sim("Conway's Game of Life");
        s.randomize(3);
        let start = s.grid().cells.clone();
        for _ in 0..5 {
            s.step();
        }
        assert_eq!(s.generation, 5);
        assert_eq!(s.timeline_len(), 6, "generations 0-5 are kept");
        assert!(s.timeline_pos().is_none(), "live until scrubbed");

        assert!(s.scrub(0));
        assert_eq!(s.generation, 0);
        assert_eq!(s.grid().cells, start);
        assert_eq!(s.timeline_pos(), Some(0));
        assert_eq!(s.timeline_generation(5), Some(5));

        assert!(s.resume_live());
        assert_eq!(s.generation, 5);
        assert!(s.timeline_pos().is_none());
    }

    #[test]
    fn stepping_after_a_scrub_forks_the_timeline() {
        let mut s = sim("Conway's Game of Life");
        s.randomize(5);
        for _ in 0..8 {
            s.step();
        }
        assert!(s.scrub(2));
        s.step();
        assert_eq!(s.generation, 3);
        assert_eq!(s.timeline_len(), 4, "future generations are dropped");
        assert!(s.timeline_pos().is_none());
    }

    #[test]
    fn timeline_is_byte_capped() {
        let mut s = sim("Conway's Game of Life");
        s.max_timeline_bytes = 20 * 20 * 2 * std::mem::size_of::<u8>();
        s.randomize(1);
        for _ in 0..10 {
            s.step();
        }
        assert!(s.timeline_len() <= 2, "old snapshots are evicted");
    }

    #[test]
    fn mutating_the_grid_clears_the_timeline() {
        let mut s = sim("Conway's Game of Life");
        s.randomize(2);
        s.step();
        assert!(s.timeline_len() > 0);
        s.reset();
        assert_eq!(s.timeline_len(), 0);
        assert!(s.timeline_pos().is_none());
    }

    #[test]
    fn frame_buffer_is_byte_capped() {
        let mut s = sim("Conway's Game of Life");
        s.max_frame_bytes = 400; // exactly one 20x20 frame
        s.record_frames = true;
        for _ in 0..5 {
            s.step();
        }
        assert_eq!(s.frame_count(), 1);
        assert!(s.frame_bytes() <= s.max_frame_bytes);
    }

    #[test]
    fn population_history_is_capped() {
        let mut s = sim("Conway's Game of Life");
        for _ in 0..(POP_HISTORY + 10) {
            s.step();
        }
        assert_eq!(s.pop_history().len(), POP_HISTORY);
    }
}
