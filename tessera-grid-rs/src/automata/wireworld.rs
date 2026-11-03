use rand::RngExt;

use crate::core::{BoundaryMode, Grid, NeighborBuffers};

use super::Automaton;

/// Wireworld – four states: Empty (0), Head (1), Tail (2), Conductor (3).
pub struct Wireworld {
    grid: Grid,
    boundary: BoundaryMode,
    neighbors: NeighborBuffers,
}

impl Wireworld {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            neighbors: NeighborBuffers::default(),
        }
    }
}

impl Wireworld {
    /// A conductor ring with one electron travelling around it.
    ///
    /// The ring is a Manhattan diamond, so every wire cell touches exactly two
    /// other wire cells: a square corner would put a wire cell diagonally next
    /// to the head, which spawns a second signal and shortens the clock.
    fn load_clock(&mut self) {
        let (w, h) = (self.grid.width, self.grid.height);
        let radius = (w.min(h) / 2).saturating_sub(1).min(6);
        if radius == 0 {
            return;
        }
        let (cx, cy) = (w / 2, h / 2);
        for y in 0..h {
            for x in 0..w {
                let dx = (x as i32 - cx as i32).unsigned_abs() as usize;
                let dy = (y as i32 - cy as i32).unsigned_abs() as usize;
                if dx + dy == radius {
                    self.grid.set(y, x, 3); // conductor
                }
            }
        }
        // Head at the top vertex, tail just clockwise of it.
        self.grid.set(cy - radius, cx, 1);
        self.grid.set(cy - radius + 1, cx + 1, 2);
    }
}

impl Automaton for Wireworld {
    fn name(&self) -> &'static str {
        "Wireworld"
    }

    fn step(&mut self) {
        let boundary = self.boundary;
        let Self {
            grid, neighbors, ..
        } = self;
        grid.neighbor_counts_into(boundary, 1, neighbors);
        let head_neighbors = &neighbors.counts;
        grid.update_cells(|i, state| match state {
            0 => 0,
            1 => 2,
            2 => 3,
            3 => {
                let hn = head_neighbors[i];
                if hn == 1 || hn == 2 {
                    1
                } else {
                    3
                }
            }
            _ => 0,
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
        match pattern {
            "Empty" => return,
            "Clock" => {
                self.load_clock();
                return;
            }
            _ => {}
        }
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            let r: f32 = rng.random();
            *cell = if r < 0.02 {
                1
            } else if r < 0.12 {
                3
            } else {
                0
            };
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Clock", "Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
    fn rule_string(&self) -> String {
        "WireWorld".to_owned()
    }

    fn max_state(&self) -> u8 {
        3
    }
    fn randomize(&mut self, seed: u64) {
        use rand::{RngExt, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let cells: Vec<u8> = (0..self.grid.cells.len())
            .map(|_| {
                let r: f32 = rng.random();
                if r < 0.02 {
                    1
                } else if r < 0.12 {
                    3
                } else {
                    0
                }
            })
            .collect();
        self.grid.cells = cells;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_count(wire: &Wireworld, state: u8) -> usize {
        wire.grid
            .cells
            .iter()
            .filter(|&&cell| cell == state)
            .count()
    }

    #[test]
    fn the_clock_ring_keeps_one_signal_circulating() {
        let mut wire = Wireworld::new(17, 17);
        wire.load_pattern("Clock");
        let initial = wire.grid.cells.clone();
        let radius = 6;
        let ring_length = 4 * radius;

        assert_eq!(state_count(&wire, 1), 1, "one head to start");
        assert_eq!(state_count(&wire, 2), 1, "one tail to start");

        for step in 1..=ring_length {
            wire.step();
            assert_eq!(state_count(&wire, 1), 1, "one head after step {step}");
            assert_eq!(state_count(&wire, 2), 1, "one tail after step {step}");
        }
        assert_eq!(
            wire.grid.cells, initial,
            "the ring returns to its starting state"
        );
    }

    #[test]
    fn the_clock_needs_room_and_degrades_safely() {
        let mut tiny = Wireworld::new(2, 2);
        tiny.load_pattern("Clock");
        assert_eq!(
            tiny.grid.population(),
            0,
            "no ring fits, so nothing is drawn"
        );
    }
}
