use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Cell state used to draw the ant on top of the binary grid.
const ANT_MARKER: u8 = 2;

/// Langton's Ant – single ant on a binary grid.
pub struct LangtonsAnt {
    /// True cell states (0/1).
    base: Grid,
    /// Display copy with ant position marked as state 2.
    display: Grid,
    pub ant_x: usize,
    pub ant_y: usize,
    /// 0 = North, 1 = East, 2 = South, 3 = West
    pub ant_dir: u8,
}

impl LangtonsAnt {
    pub fn new(width: usize, height: usize) -> Self {
        let mut s = Self {
            base: Grid::new(width, height),
            display: Grid::new(width, height),
            ant_x: width / 2,
            ant_y: height / 2,
            ant_dir: 0,
        };
        s.sync_display();
        s
    }

    fn sync_display(&mut self) {
        self.display.cells.clone_from(&self.base.cells);
        self.display.set(self.ant_y, self.ant_x, ANT_MARKER);
    }

    /// Reinstate the display value of one cell from the base grid.
    fn restore_display_cell(&mut self, y: usize, x: usize) {
        let value = self.base.get(y, x);
        self.display.set(y, x, value);
    }

    fn encode_aux(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(17);
        out.extend_from_slice(&(self.ant_x as u64).to_le_bytes());
        out.extend_from_slice(&(self.ant_y as u64).to_le_bytes());
        out.push(self.ant_dir);
        out
    }
}

impl Automaton for LangtonsAnt {
    fn name(&self) -> &'static str {
        "Langton's Ant"
    }

    fn step(&mut self) {
        let current = self.base.get(self.ant_y, self.ant_x);
        self.base.set(self.ant_y, self.ant_x, 1 - current);

        if current == 0 {
            self.ant_dir = (self.ant_dir + 1) % 4;
        } else {
            self.ant_dir = (self.ant_dir + 3) % 4;
        }

        let h = self.base.height;
        let w = self.base.width;
        let (from_y, from_x) = (self.ant_y, self.ant_x);
        match self.ant_dir {
            0 => self.ant_y = (self.ant_y + h - 1) % h,
            1 => self.ant_x = (self.ant_x + 1) % w,
            2 => self.ant_y = (self.ant_y + 1) % h,
            _ => self.ant_x = (self.ant_x + w - 1) % w,
        }

        // Only the vacated cell and the ant's new cell change, so updating them
        // directly keeps a step O(1) instead of copying the whole grid.
        self.restore_display_cell(from_y, from_x);
        self.display.set(self.ant_y, self.ant_x, ANT_MARKER);
    }

    fn reset(&mut self) {
        self.base.clear();
        self.ant_x = self.base.width / 2;
        self.ant_y = self.base.height / 2;
        self.ant_dir = 0;
        self.sync_display();
    }

    fn get_grid(&self) -> &Grid {
        &self.display
    }

    fn get_grid_mut(&mut self) -> &mut Grid {
        &mut self.base
    }

    fn load_pattern(&mut self, _pattern: &str) {
        self.reset();
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        BoundaryMode::Wrap
    }

    fn set_boundary(&mut self, _mode: BoundaryMode) {}
    fn set_cells(&mut self, cells: &[u8]) {
        if self.base.cells.len() == cells.len() {
            self.base.cells.copy_from_slice(cells);
            self.sync_display();
        }
    }

    fn edit_cells(&mut self, edit: &mut dyn FnMut(&mut [u8])) {
        edit(&mut self.base.cells);
        self.sync_display();
    }

    fn paint(&mut self, y: usize, x: usize, state: u8) {
        if y < self.base.height && x < self.base.width {
            self.base.set(y, x, state.min(1));
            self.sync_display();
        }
    }

    fn rule_string(&self) -> String {
        "LangtonsAnt".to_owned()
    }

    fn supports_boundary(&self) -> bool {
        false
    }
    fn randomize(&mut self, _seed: u64) {
        self.reset();
    }

    fn state_cells(&self) -> Vec<u8> {
        // `get_grid` returns the display grid, which carries the ant marker.
        self.base.cells.clone()
    }

    fn aux_state(&self) -> Vec<u8> {
        self.encode_aux()
    }

    fn set_aux_state(&mut self, state: &[u8]) {
        if state.len() != 17 {
            return;
        }
        let x = u64::from_le_bytes(state[0..8].try_into().unwrap_or_default());
        let y = u64::from_le_bytes(state[8..16].try_into().unwrap_or_default());
        let dir = state[16];
        let (w, h) = (self.base.width, self.base.height);
        if w == 0 || h == 0 || dir > 3 {
            return;
        }
        self.ant_x = (x as usize) % w;
        self.ant_y = (y as usize) % h;
        self.ant_dir = dir;
        self.sync_display();
    }
}
