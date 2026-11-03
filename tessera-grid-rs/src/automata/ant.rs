use crate::core::{BoundaryMode, Grid};

use super::Automaton;

/// Cell state used to draw the ant on top of the grid.
const ANT_MARKER: u8 = 2;

/// Langton's Ant – single ant on a binary grid.
///
/// The grid holds only the real cell states. The ant's position is exposed
/// through [`Automaton::marker`] and drawn as an overlay, so it can never leak
/// into stepping, undo, RLE exports or saved state files.
pub struct LangtonsAnt {
    grid: Grid,
    pub ant_x: usize,
    pub ant_y: usize,
    /// 0 = North, 1 = East, 2 = South, 3 = West
    pub ant_dir: u8,
}

impl LangtonsAnt {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            ant_x: width / 2,
            ant_y: height / 2,
            ant_dir: 0,
        }
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
        let (y, x) = (self.ant_y, self.ant_x);
        let alive = self.grid.get(y, x) != 0;
        self.grid.set(y, x, u8::from(!alive));

        if alive {
            self.ant_dir = (self.ant_dir + 3) % 4;
        } else {
            self.ant_dir = (self.ant_dir + 1) % 4;
        }

        let h = self.grid.height.max(1);
        let w = self.grid.width.max(1);
        match self.ant_dir {
            0 => self.ant_y = (self.ant_y + h - 1) % h,
            1 => self.ant_x = (self.ant_x + 1) % w,
            2 => self.ant_y = (self.ant_y + 1) % h,
            _ => self.ant_x = (self.ant_x + w - 1) % w,
        }
    }

    fn reset(&mut self) {
        self.grid.clear();
        self.ant_x = self.grid.width / 2;
        self.ant_y = self.grid.height / 2;
        self.ant_dir = 0;
    }

    fn get_grid(&self) -> &Grid {
        &self.grid
    }

    fn get_grid_mut(&mut self) -> &mut Grid {
        &mut self.grid
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

    fn rule_string(&self) -> String {
        "LangtonsAnt".to_owned()
    }

    fn supports_boundary(&self) -> bool {
        false
    }

    fn randomize(&mut self, _seed: u64) {
        self.reset();
    }

    fn marker(&self) -> Option<(usize, u8)> {
        let (w, h) = (self.grid.width, self.grid.height);
        (w > 0 && h > 0 && self.ant_y < h && self.ant_x < w)
            .then_some((self.ant_y * w + self.ant_x, ANT_MARKER))
    }

    fn aux_state(&self) -> Vec<u8> {
        self.encode_aux()
    }

    fn validate_aux_state(&self, state: &[u8]) -> Result<(), String> {
        if state.len() != 17 {
            return Err("Ant state needs 17 bytes for position and direction".to_owned());
        }
        let x = u64::from_le_bytes(state[0..8].try_into().expect("8-byte x"));
        let y = u64::from_le_bytes(state[8..16].try_into().expect("8-byte y"));
        if x >= self.grid.width as u64 || y >= self.grid.height as u64 || state[16] > 3 {
            return Err("Ant position or direction is outside its valid range".to_owned());
        }
        Ok(())
    }

    fn set_aux_state(&mut self, state: &[u8]) {
        if state.len() != 17 {
            return;
        }
        let x = u64::from_le_bytes(state[0..8].try_into().unwrap_or_default());
        let y = u64::from_le_bytes(state[8..16].try_into().unwrap_or_default());
        let dir = state[16];
        let (w, h) = (self.grid.width, self.grid.height);
        if w == 0 || h == 0 || dir > 3 {
            return;
        }
        self.ant_x = (x as usize) % w;
        self.ant_y = (y as usize) % h;
        self.ant_dir = dir;
    }
}
