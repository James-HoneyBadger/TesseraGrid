use crate::core::{BoundaryMode, Grid};

use super::Automaton;

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
        self.display.set(self.ant_y, self.ant_x, 2);
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
        match self.ant_dir {
            0 => self.ant_y = (self.ant_y + h - 1) % h,
            1 => self.ant_x = (self.ant_x + 1) % w,
            2 => self.ant_y = (self.ant_y + 1) % h,
            _ => self.ant_x = (self.ant_x + w - 1) % w,
        }

        self.sync_display();
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
}
