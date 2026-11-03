//! Automaton implementations behind a single [`Automaton`] trait.

pub mod ant;
pub mod briansbrain;
pub mod conway;
pub mod elementary;
pub mod generations;
pub mod hexagonal;
pub mod highlife;
pub mod immigration;
pub mod lifelike;
pub mod rainbow;
pub mod wireworld;

#[cfg(test)]
mod tests;

pub use ant::LangtonsAnt;
pub use briansbrain::BriansBrain;
pub use conway::ConwayLife;
pub use elementary::Elementary1D;
pub use generations::Generations;
pub use hexagonal::HexagonalLife;
pub use highlife::HighLife;
pub use immigration::ImmigrationGame;
pub use lifelike::LifeLike;
pub use rainbow::RainbowGame;
pub use wireworld::Wireworld;

use crate::core::{BoundaryMode, Grid};

// ---------------------------------------------------------------------------
// Automaton trait
// ---------------------------------------------------------------------------

/// Common interface for all cellular automata.
///
/// Mutating the grid is deliberately limited to [`Automaton::set_cells`],
/// [`Automaton::edit_cells`] and [`Automaton::paint`] so that automata which keep
/// auxiliary state (for example Langton's Ant) can stay in sync.
pub trait Automaton {
    fn name(&self) -> &'static str;
    fn step(&mut self);
    fn reset(&mut self);
    fn get_grid(&self) -> &Grid;
    /// Raw mutable access used only by this trait's default implementations.
    ///
    /// Callers elsewhere must use [`Automaton::set_cells`],
    /// [`Automaton::edit_cells`] or [`Automaton::paint`]; writing through this
    /// handle bypasses the resynchronisation those perform.
    #[doc(hidden)]
    fn get_grid_mut(&mut self) -> &mut Grid;
    fn load_pattern(&mut self, pattern: &str);
    fn available_patterns(&self) -> &'static [&'static str];
    fn boundary(&self) -> BoundaryMode;
    fn set_boundary(&mut self, mode: BoundaryMode);

    /// Rule notation for the active mode, used for RLE headers and the UI.
    fn rule_string(&self) -> String {
        "B3/S23".to_owned()
    }

    /// Highest cell state that can be painted in this mode.
    fn max_state(&self) -> u8 {
        1
    }

    /// `false` when the automaton ignores the boundary mode.
    fn supports_boundary(&self) -> bool {
        true
    }

    /// Canonical simulation state, excluding render-only decoration.
    ///
    /// [`Automaton::get_grid`] may include visual markers (Langton's Ant draws
    /// itself as state 2). Undo and timeline snapshots must store what the
    /// automaton steps from instead, or restoring a snapshot would feed render
    /// artefacts back into the simulation.
    fn state_cells(&self) -> Vec<u8> {
        self.get_grid().cells.clone()
    }

    /// Opaque state that lives outside the grid, captured by undo snapshots.
    ///
    /// Automata that keep extra state (for example Langton's Ant heading) must
    /// round-trip it here, otherwise undo would restore cells but not the
    /// automaton that operates on them.
    fn aux_state(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Restore state produced by [`Automaton::aux_state`].
    fn set_aux_state(&mut self, _state: &[u8]) {}

    /// Fill the grid with a reproducible pseudo-random population.
    fn randomize(&mut self, seed: u64) {
        use rand::{RngExt, SeedableRng};
        let max_state = self.max_state().max(1);
        let width = self.get_grid().width;
        let height = self.get_grid().height;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let cells: Vec<u8> = (0..width * height)
            .map(|_| {
                if rng.random::<f32>() < 0.15 {
                    rng.random_range(1..=max_state)
                } else {
                    0
                }
            })
            .collect();
        self.set_cells(&cells);
    }

    /// Replace every cell state, clamping to [`Automaton::max_state`].
    ///
    /// Clamping here protects every entry point that feeds the grid from the
    /// outside world (state files, clipboard paste, RLE import): a state the
    /// automaton cannot represent would otherwise reach its step function.
    fn set_cells(&mut self, cells: &[u8]) {
        let max = self.max_state();
        let grid = self.get_grid_mut();
        if grid.cells.len() == cells.len() {
            for (dst, &src) in grid.cells.iter_mut().zip(cells) {
                *dst = src.min(max);
            }
        }
    }

    /// Mutate the cell buffer in place, clamping the result to
    /// [`Automaton::max_state`].
    fn edit_cells(&mut self, edit: &mut dyn FnMut(&mut [u8])) {
        let max = self.max_state();
        let cells = &mut self.get_grid_mut().cells;
        edit(cells);
        for cell in cells.iter_mut() {
            *cell = (*cell).min(max);
        }
    }

    /// Paint a single cell without changing the grid size.
    fn paint(&mut self, y: usize, x: usize, state: u8) {
        let max = self.max_state();
        let (h, w) = {
            let grid = self.get_grid();
            (grid.height, grid.width)
        };
        if y < h && x < w {
            self.get_grid_mut().set(y, x, state.min(max));
        }
    }

    /// Replace the B/S rule set of a configurable automaton.
    ///
    /// Returns `false` when the automaton has fixed rules, so callers know they
    /// must rebuild instead. Applying rules in place keeps the grid and the undo
    /// history intact while the user edits the rule string.
    fn set_rules(&mut self, _birth: &[u8], _survival: &[u8]) -> bool {
        false
    }

    /// Row range holding the cells the automaton actually evolves.
    ///
    /// Space-time automata such as [`Elementary1D`] keep history in the grid;
    /// only the newest row should be counted as population or aged. The default
    /// treats the whole grid as live state.
    fn active_rows(&self) -> (usize, usize) {
        (0, self.get_grid().height)
    }

    /// Index and state of a render-only marker drawn on top of the grid.
    ///
    /// Langton's Ant uses this to show its position without storing it in the
    /// cell buffer: the marker is a *view* of the automaton, never part of the
    /// state that stepping, undo, RLE export or `.tessera` files see.
    fn marker(&self) -> Option<(usize, u8)> {
        None
    }
}

/// Canonical mode name for the UI and config files.
///
/// Unknown names (hand-edited configs, damaged state files) resolve to Conway
/// instead of leaving the UI showing a mode the engine never built.
pub fn canonical_mode_name(mode: &str) -> &'static str {
    Mode::from_name(mode).map_or(Mode::Conway.name(), Mode::name)
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Every automaton the simulator can build, in display order.
///
/// A single enum keeps mode dispatch exhaustive: adding a variant forces every
/// `match` to be updated instead of silently falling through to Conway. The
/// human-readable names are the strings used by the UI, the CLI and saved
/// configs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Conway,
    HighLife,
    Hexagonal,
    Elementary1D,
    Immigration,
    Rainbow,
    Ant,
    Wireworld,
    BriansBrain,
    Generations,
    Custom,
}

impl Mode {
    pub const ALL: [Mode; 11] = [
        Mode::Conway,
        Mode::HighLife,
        Mode::Hexagonal,
        Mode::Elementary1D,
        Mode::Immigration,
        Mode::Rainbow,
        Mode::Ant,
        Mode::Wireworld,
        Mode::BriansBrain,
        Mode::Generations,
        Mode::Custom,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Conway => "Conway's Game of Life",
            Mode::HighLife => "High Life",
            Mode::Hexagonal => "Hexagonal Life",
            Mode::Elementary1D => "Elementary 1D",
            Mode::Immigration => "Immigration Game",
            Mode::Rainbow => "Rainbow Game",
            Mode::Ant => "Langton's Ant",
            Mode::Wireworld => "Wireworld",
            Mode::BriansBrain => "Brian's Brain",
            Mode::Generations => "Generations",
            Mode::Custom => "Custom Rules",
        }
    }

    /// Short human-readable description, shown in the sidebar.
    pub fn description(self) -> &'static str {
        match self {
            Mode::Conway => "Classic B3/S23 Life on a square grid.",
            Mode::HighLife => "B36/S23 — adds a replicator to Conway's rules.",
            Mode::Hexagonal => "B2/S34 on a hexagonal (odd-r) neighbourhood.",
            Mode::Elementary1D => {
                "Wolfram's 256 one-dimensional rules, drawn as a space-time diagram. Rule 110 is Turing complete."
            }
            Mode::Immigration => "Three-colour Life; newborns inherit the majority colour of their parents.",
            Mode::Rainbow => "Six-colour Life; newborns blend the average colour of their parents.",
            Mode::Ant => "A single ant turns, flips cells, and walks the grid.",
            Mode::Wireworld => "Four states: empty, head, tail, and conductor.",
            Mode::BriansBrain => "Three states: off, firing, and refractory.",
            Mode::Generations => "Life-like rules with fading decay states.",
            Mode::Custom => "Any Life-like rule written in B/S notation.",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// `true` for modes whose B/S rule is supplied by the user.
    pub fn uses_custom_rules(self) -> bool {
        matches!(self, Mode::Custom)
    }

    /// Build the automaton for this mode.
    pub fn build(self, width: usize, height: usize) -> Box<dyn Automaton> {
        match self {
            Mode::Conway => Box::new(ConwayLife::new(width, height)),
            Mode::HighLife => Box::new(HighLife::new(width, height)),
            Mode::Hexagonal => Box::new(HexagonalLife::new(width, height)),
            Mode::Elementary1D => {
                Box::new(Elementary1D::new(width, height, elementary::DEFAULT_RULE))
            }
            Mode::Immigration => Box::new(ImmigrationGame::new(width, height)),
            Mode::Rainbow => Box::new(RainbowGame::new(width, height)),
            Mode::Ant => Box::new(LangtonsAnt::new(width, height)),
            Mode::Wireworld => Box::new(Wireworld::new(width, height)),
            Mode::BriansBrain => Box::new(BriansBrain::new(width, height)),
            Mode::Generations => Box::new(Generations::new(width, height)),
            Mode::Custom => Box::new(LifeLike::new(width, height, vec![3], vec![2, 3])),
        }
    }
}

/// All supported automaton modes in display order.
pub const ALL_MODES: &[&str] = &[
    "Conway's Game of Life",
    "High Life",
    "Hexagonal Life",
    "Elementary 1D",
    "Immigration Game",
    "Rainbow Game",
    "Langton's Ant",
    "Wireworld",
    "Brian's Brain",
    "Generations",
    "Custom Rules",
];

/// Short human-readable description for a mode name, shown in the sidebar.
pub fn description(mode: &str) -> &'static str {
    Mode::from_name(mode).map_or("", Mode::description)
}

/// Build a boxed automaton for the given mode string.
///
/// Unknown names fall back to Conway, which only matters for hand-edited config
/// files: the UI and CLI both validate against [`ALL_MODES`] first.
pub fn make_automaton(mode: &str, width: usize, height: usize) -> Box<dyn Automaton> {
    Mode::from_name(mode)
        .unwrap_or(Mode::Conway)
        .build(width, height)
}
