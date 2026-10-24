pub mod ant;
pub mod briansbrain;
pub mod conway;
pub mod generations;
pub mod hexagonal;
pub mod highlife;
pub mod immigration;
pub mod lifelike;
pub mod rainbow;
pub mod wireworld;

pub use ant::LangtonsAnt;
pub use briansbrain::BriansBrain;
pub use conway::ConwayLife;
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
/// auxiliary state (for example Langton's Ant) can stay in sync. `get_grid_mut`
/// exists only for the default implementations; callers outside this module
/// must use the safe helpers.
pub trait Automaton {
    fn name(&self) -> &'static str;
    fn step(&mut self);
    fn reset(&mut self);
    fn get_grid(&self) -> &Grid;
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

    /// Replace every cell state.
    fn set_cells(&mut self, cells: &[u8]) {
        let grid = self.get_grid_mut();
        if grid.cells.len() == cells.len() {
            grid.cells.copy_from_slice(cells);
        }
    }

    /// Mutate the cell buffer in place.
    fn edit_cells(&mut self, edit: &mut dyn FnMut(&mut [u8])) {
        edit(&mut self.get_grid_mut().cells);
    }

    /// Paint a single cell without changing the grid size.
    fn paint(&mut self, y: usize, x: usize, state: u8) {
        let (h, w) = {
            let grid = self.get_grid();
            (grid.height, grid.width)
        };
        if y < h && x < w {
            self.get_grid_mut().set(y, x, state);
        }
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// All supported automaton modes in display order.
pub const ALL_MODES: &[&str] = &[
    "Conway's Game of Life",
    "High Life",
    "Hexagonal Life",
    "Immigration Game",
    "Rainbow Game",
    "Langton's Ant",
    "Wireworld",
    "Brian's Brain",
    "Generations",
    "Custom Rules",
];

/// Short human-readable description for each mode, shown in the sidebar.
pub fn description(mode: &str) -> &'static str {
    match mode {
        "Conway's Game of Life" => "Classic B3/S23 Life on a square grid.",
        "High Life" => "B36/S23 — adds a replicator to Conway's rules.",
        "Hexagonal Life" => "B2/S34 on a hexagonal (odd-r) neighbourhood.",
        "Immigration Game" => "Two-colour Life; newborns inherit the majority colour.",
        "Rainbow Game" => "Six-colour cyclic variant of Life.",
        "Langton's Ant" => "A single ant turns, flips cells, and walks the grid.",
        "Wireworld" => "Four states: empty, head, tail, and conductor.",
        "Brian's Brain" => "Three states: off, firing, and refractory.",
        "Generations" => "Life-like rules with fading decay states.",
        "Custom Rules" => "Any Life-like rule written in B/S notation.",
        _ => "",
    }
}

/// Build a boxed automaton for the given mode string.
pub fn make_automaton(mode: &str, width: usize, height: usize) -> Box<dyn Automaton> {
    match mode {
        "Conway's Game of Life" => Box::new(ConwayLife::new(width, height)),
        "High Life" => Box::new(HighLife::new(width, height)),
        "Hexagonal Life" => Box::new(HexagonalLife::new(width, height)),
        "Immigration Game" => Box::new(ImmigrationGame::new(width, height)),
        "Rainbow Game" => Box::new(RainbowGame::new(width, height)),
        "Langton's Ant" => Box::new(LangtonsAnt::new(width, height)),
        "Wireworld" => Box::new(Wireworld::new(width, height)),
        "Brian's Brain" => Box::new(BriansBrain::new(width, height)),
        "Generations" => Box::new(Generations::new(width, height)),
        "Custom Rules" => Box::new(LifeLike::new(width, height, vec![3], vec![2, 3])),
        _ => Box::new(ConwayLife::new(width, height)),
    }
}
