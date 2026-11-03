use rand::RngExt;

use crate::core::{BoundaryMode, Grid, NeighborBuffers};

use super::Automaton;

/// Immigration Game – multi-color Conway variant (states 1, 2, 3).
pub struct ImmigrationGame {
    grid: Grid,
    boundary: BoundaryMode,
    /// Total live neighbours and their colour sum, per cell.
    neighbors: NeighborBuffers,
    /// Neighbours in colour 1, per cell.
    colour_one: NeighborBuffers,
    /// Neighbours in colour 2, per cell.
    colour_two: NeighborBuffers,
}

impl ImmigrationGame {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            boundary: BoundaryMode::Wrap,
            neighbors: NeighborBuffers::default(),
            colour_one: NeighborBuffers::default(),
            colour_two: NeighborBuffers::default(),
        }
    }

    /// Colour a newborn takes from exactly three parents.
    ///
    /// The majority wins; when the three parents all differ the lowest colour is
    /// used, which keeps the rule deterministic. Callers only ask when there are
    /// three neighbours, so the third colour's count is what remains.
    fn majority_colour(ones: u8, twos: u8, total: u8) -> u8 {
        let threes = total.saturating_sub(ones).saturating_sub(twos);
        if ones >= 2 {
            1
        } else if twos >= 2 {
            2
        } else if threes >= 2 {
            3
        } else {
            1
        }
    }
}

impl Automaton for ImmigrationGame {
    fn name(&self) -> &'static str {
        "Immigration Game"
    }

    fn step(&mut self) {
        let boundary = self.boundary;
        let Self {
            grid,
            neighbors,
            colour_one,
            colour_two,
            ..
        } = self;
        grid.neighbor_stats_into(boundary, neighbors);
        grid.neighbor_counts_into(boundary, 1, colour_one);
        grid.neighbor_counts_into(boundary, 2, colour_two);

        let total = &neighbors.counts;
        let ones = &colour_one.counts;
        let twos = &colour_two.counts;
        grid.update_cells(|i, state| {
            let live = total[i];
            if state > 0 {
                if live == 2 || live == 3 {
                    state
                } else {
                    0
                }
            } else if live == 3 {
                Self::majority_colour(ones[i], twos[i], live)
            } else {
                0
            }
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
        let w = self.grid.width;
        let h = self.grid.height;
        let cx = (w / 2) as i32;
        let cy = (h / 2) as i32;

        match pattern {
            "Empty" => {}
            "Color Mix" => {
                for &(dx, dy) in &[(1i32, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
                    let x = cx - 20 + dx;
                    let y = cy - 15 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        self.grid.set(y as usize, x as usize, 1);
                    }
                }
                for &(dx, dy) in &[(0i32, 0), (1, 0), (2, 0)] {
                    let x = cx + dx - 1;
                    let y = cy - 15 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        self.grid.set(y as usize, x as usize, 2);
                    }
                }
            }
            _ => {
                let mut rng = rand::rng();
                for cell in self.grid.cells.iter_mut() {
                    if rng.random::<f32>() < 0.15 {
                        *cell = rng.random_range(1..=3);
                    }
                }
            }
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Color Mix", "Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        self.boundary
    }

    fn set_boundary(&mut self, mode: BoundaryMode) {
        self.boundary = mode;
    }
    fn max_state(&self) -> u8 {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(cells: &[(usize, usize, u8)]) -> ImmigrationGame {
        let mut game = ImmigrationGame::new(9, 9);
        for &(x, y, state) in cells {
            game.paint(y, x, state);
        }
        game
    }

    fn centre_after_step(game: &mut ImmigrationGame) -> u8 {
        game.step();
        game.get_grid().get(4, 4)
    }

    #[test]
    fn newborns_inherit_the_majority_colour() {
        // Three parents: two of colour 1, one of colour 2.
        let mut two_ones = game(&[(3, 4, 1), (4, 3, 1), (5, 4, 2)]);
        assert_eq!(centre_after_step(&mut two_ones), 1);

        // Two of colour 2, one of colour 1.
        let mut two_twos = game(&[(3, 4, 2), (4, 3, 2), (5, 4, 1)]);
        assert_eq!(centre_after_step(&mut two_twos), 2);

        // Two of colour 3, one of colour 2.
        let mut two_threes = game(&[(3, 4, 3), (4, 3, 3), (5, 4, 2)]);
        assert_eq!(centre_after_step(&mut two_threes), 3);

        // Three identical parents.
        let mut all_threes = game(&[(3, 4, 3), (4, 3, 3), (5, 4, 3)]);
        assert_eq!(centre_after_step(&mut all_threes), 3);
    }

    #[test]
    fn a_three_way_tie_is_deterministic() {
        let mut tie = game(&[(3, 4, 1), (4, 3, 2), (5, 4, 3)]);
        assert_eq!(centre_after_step(&mut tie), 1, "ties resolve to colour 1");
    }

    #[test]
    fn survivors_keep_their_colour_and_lonely_cells_die() {
        let mut stable = game(&[(3, 4, 2), (4, 3, 2), (5, 4, 2), (4, 3, 2)]);
        stable.step();
        // The cell at x = 4, y = 3 keeps its colour.
        assert_eq!(stable.get_grid().get(3, 4), 2, "survivor keeps its colour");

        let mut lonely = game(&[(4, 4, 3)]);
        lonely.step();
        assert_eq!(lonely.get_grid().get(4, 4), 0, "underpopulation kills");
    }

    #[test]
    fn births_need_exactly_three_neighbours() {
        let mut two = game(&[(3, 4, 1), (5, 4, 1)]);
        two.step();
        assert_eq!(two.get_grid().get(4, 4), 0);

        let mut four = game(&[(3, 4, 1), (5, 4, 1), (4, 3, 1), (4, 5, 1)]);
        four.step();
        assert_eq!(four.get_grid().get(4, 4), 0);
    }
}
