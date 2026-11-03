use rand::RngExt;

use crate::core::{BoundaryMode, Grid, PARALLEL_CELL_THRESHOLD};

use super::Automaton;

/// Hexagonal Game of Life – B2/S34 on an offset-coordinate grid.
///
/// Uses "Odd-r" horizontal layout for neighbour calculation.
pub struct HexagonalLife {
    grid: Grid,
    /// Wrapped left/right column index for every x.
    cols: WrapWindow,
    /// Wrapped above/below row index for every y.
    rows: WrapWindow,
    /// Neighbour counts, reused between steps.
    counts: Vec<u8>,
}

impl HexagonalLife {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: Grid::new(width, height),
            cols: WrapWindow::new(width),
            rows: WrapWindow::new(height),
            counts: Vec::new(),
        }
    }

    /// Count the six hex neighbours of every cell.
    ///
    /// The offset tables are precomputed once per resize, so unlike the naive
    /// version this does no `rem_euclid` in the inner loop.
    fn count_neighbours(grid: &Grid, cols: &WrapWindow, rows: &WrapWindow, counts: &mut Vec<u8>) {
        let (w, h) = (grid.width, grid.height);
        counts.resize(w * h, 0);
        if w == 0 || h == 0 {
            return;
        }
        let cells = &grid.cells;

        let compute_row = |y: usize, out: &mut [u8]| {
            let (up, down, here) = (rows.prev[y] * w, rows.next[y] * w, y * w);
            let even = y.is_multiple_of(2);
            for x in 0..w {
                let (c1, c2) = if even {
                    (cols.prev[x], x)
                } else {
                    (x, cols.next[x])
                };
                let mut n = u8::from(cells[here + cols.prev[x]] > 0);
                n += u8::from(cells[here + cols.next[x]] > 0);
                n += u8::from(cells[up + c1] > 0);
                n += u8::from(cells[up + c2] > 0);
                n += u8::from(cells[down + c1] > 0);
                n += u8::from(cells[down + c2] > 0);
                out[x] = n;
            }
        };

        if w * h >= PARALLEL_CELL_THRESHOLD {
            use rayon::prelude::*;
            counts
                .par_chunks_mut(w)
                .enumerate()
                .for_each(|(y, out)| compute_row(y, out));
        } else {
            for (y, out) in counts.chunks_mut(w).enumerate() {
                compute_row(y, out);
            }
        }
    }
}

/// Wrapped neighbour indices along an axis that always wraps.
struct WrapWindow {
    prev: Vec<usize>,
    next: Vec<usize>,
}

impl WrapWindow {
    fn new(size: usize) -> Self {
        let modulo = size.max(1);
        Self {
            prev: (0..size).map(|i| (i + size - 1) % modulo).collect(),
            next: (0..size).map(|i| (i + 1) % modulo).collect(),
        }
    }

    fn matches(&self, size: usize) -> bool {
        self.prev.len() == size
    }
}

impl Automaton for HexagonalLife {
    fn name(&self) -> &'static str {
        "Hexagonal Life"
    }

    fn step(&mut self) {
        let (w, h) = (self.grid.width, self.grid.height);
        if !self.cols.matches(w) {
            self.cols = WrapWindow::new(w);
        }
        if !self.rows.matches(h) {
            self.rows = WrapWindow::new(h);
        }

        let Self {
            grid,
            cols,
            rows,
            counts,
        } = self;
        Self::count_neighbours(grid, cols, rows, counts);

        let counts = &counts[..];
        grid.update_cells(|i, state| {
            let n = counts[i];
            if state > 0 {
                u8::from(n == 3 || n == 4)
            } else {
                u8::from(n == 2)
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
        if pattern == "Empty" {
            return;
        }
        let mut rng = rand::rng();
        for cell in self.grid.cells.iter_mut() {
            *cell = u8::from(rng.random::<f32>() < 0.15);
        }
    }

    fn available_patterns(&self) -> &'static [&'static str] {
        &["Random Soup", "Empty"]
    }

    fn boundary(&self) -> BoundaryMode {
        BoundaryMode::Wrap
    }

    fn set_boundary(&mut self, _mode: BoundaryMode) {}
    fn rule_string(&self) -> String {
        "B2/S34H".to_owned()
    }

    fn supports_boundary(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference implementation: resolves each of the six neighbours with
    /// `rem_euclid` for the odd-r offset layout.
    fn reference_counts(cells: &[u8], w: usize, h: usize) -> Vec<u8> {
        const OFFSETS_EVEN: &[(i32, i32)] = &[(-1, 0), (1, 0), (-1, -1), (0, -1), (-1, 1), (0, 1)];
        const OFFSETS_ODD: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, -1), (1, -1), (0, 1), (1, 1)];
        let mut out = vec![0u8; w * h];
        for y in 0..h {
            let offsets = if y.is_multiple_of(2) {
                OFFSETS_EVEN
            } else {
                OFFSETS_ODD
            };
            for x in 0..w {
                let mut count = 0u8;
                for &(dx, dy) in offsets {
                    let nx = (x as i32 + dx).rem_euclid(w as i32) as usize;
                    let ny = (y as i32 + dy).rem_euclid(h as i32) as usize;
                    count += u8::from(cells[ny * w + nx] > 0);
                }
                out[y * w + x] = count;
            }
        }
        out
    }

    fn filled(width: usize, height: usize) -> Grid {
        let mut grid = Grid::new(width, height);
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for cell in grid.cells.iter_mut() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *cell = u8::from(state.is_multiple_of(3));
        }
        grid
    }

    #[test]
    fn windowed_counts_match_the_reference_implementation() {
        for (w, h) in [
            (1, 1),
            (1, 5),
            (5, 1),
            (2, 2),
            (3, 4),
            (4, 3),
            (9, 9),
            (33, 17),
        ] {
            let grid = filled(w, h);
            let cols = WrapWindow::new(w);
            let rows = WrapWindow::new(h);
            let mut counts = Vec::new();
            HexagonalLife::count_neighbours(&grid, &cols, &rows, &mut counts);
            assert_eq!(counts, reference_counts(&grid.cells, w, h), "{w}x{h}");
        }
    }

    #[test]
    fn windowed_counts_match_the_reference_above_the_parallel_threshold() {
        let (w, h) = (300, 300);
        let grid = filled(w, h);
        assert!(w * h >= PARALLEL_CELL_THRESHOLD);
        let mut counts = Vec::new();
        HexagonalLife::count_neighbours(
            &grid,
            &WrapWindow::new(w),
            &WrapWindow::new(h),
            &mut counts,
        );
        assert_eq!(counts, reference_counts(&grid.cells, w, h));
    }
}
