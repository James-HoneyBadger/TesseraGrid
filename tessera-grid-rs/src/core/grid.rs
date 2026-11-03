//! Flat row-major cell grid with neighbour counting.

use super::BoundaryMode;

/// Cell count above which per-cell work is spread across threads.
pub const PARALLEL_CELL_THRESHOLD: usize = 50_000;

/// A flat, heap-allocated 2-D grid of `u8` cells.
///
/// Indices are row-major: `cells[y * width + x]`.
#[derive(Clone)]
pub struct Grid {
    pub cells: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![0; width * height],
            width,
            height,
        }
    }

    #[inline]
    pub fn get(&self, y: usize, x: usize) -> u8 {
        self.cells[y * self.width + x]
    }

    #[inline]
    pub fn set(&mut self, y: usize, x: usize, val: u8) {
        self.cells[y * self.width + x] = val;
    }

    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Number of non-zero cells.
    pub fn population(&self) -> usize {
        self.cells.iter().filter(|&&c| c > 0).count()
    }

    // -----------------------------------------------------------------------
    // Neighbour helpers
    // -----------------------------------------------------------------------

    /// Map every cell index to a value, in parallel for large grids.
    ///
    /// `f` must only read the grid, so the results can be computed before the
    /// grid is mutated.
    pub fn map_cells<R: Send>(&self, f: impl Fn(usize) -> R + Sync + Send) -> Vec<R> {
        let n = self.cells.len();
        if n >= PARALLEL_CELL_THRESHOLD {
            use rayon::prelude::*;
            (0..n).into_par_iter().map(f).collect()
        } else {
            (0..n).map(f).collect()
        }
    }

    /// Recompute every cell in place from its own current value.
    ///
    /// `update` receives the cell index and its state; automata must precompute
    /// neighbour data with [`Grid::map_cells`] (or a neighbour helper) first,
    /// because cells are rewritten one index at a time. Large grids are updated
    /// in parallel.
    pub fn update_cells(&mut self, update: impl Fn(usize, u8) -> u8 + Sync + Send) {
        if self.cells.len() >= PARALLEL_CELL_THRESHOLD {
            use rayon::prelude::*;
            self.cells
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cell)| *cell = update(i, *cell));
        } else {
            for (i, cell) in self.cells.iter_mut().enumerate() {
                *cell = update(i, *cell);
            }
        }
    }

    /// Compute the count of 8-neighbours whose value equals `check_val` for
    /// every cell, returning a flat `Vec<u8>` with the same layout.
    ///
    /// Convenience wrapper around [`Grid::neighbor_counts_into`] that allocates
    /// fresh buffers; the hot path should reuse [`NeighborBuffers`].
    pub fn neighbor_counts(&self, boundary: BoundaryMode, check_val: u8) -> Vec<u8> {
        let mut buf = NeighborBuffers::default();
        self.neighbor_counts_into(boundary, check_val, &mut buf);
        buf.counts
    }

    /// Compute the count of 8-neighbours whose value equals `check_val` for
    /// every cell, writing into `buf.counts`.
    ///
    /// The grid is traversed row by row in two additive passes (horizontal,
    /// then vertical) so the inner loops contain no per-cell division, modulo or
    /// boundary branching. This is roughly five times faster than resolving
    /// every neighbour coordinate individually.
    pub fn neighbor_counts_into(
        &self,
        boundary: BoundaryMode,
        check_val: u8,
        buf: &mut NeighborBuffers,
    ) {
        let (w, h) = (self.width, self.height);
        buf.counts.clear();
        buf.row_totals.clear();
        if w == 0 || h == 0 {
            return;
        }
        let cols = AxisWindow::new(w, boundary);
        let rows = AxisWindow::new(h, boundary);

        // Pass 1: sum the three horizontal neighbours of every cell.
        buf.row_totals.resize(w * h, 0);
        for_each_row_pair(&mut buf.row_totals, &self.cells, w, |out, row| {
            for x in 0..w {
                let mut n = u8::from(row[x] == check_val);
                if let Some(l) = cols.prev[x] {
                    n += u8::from(row[l] == check_val);
                }
                if let Some(r) = cols.next[x] {
                    n += u8::from(row[r] == check_val);
                }
                out[x] = n;
            }
        });

        // Pass 2: add the horizontal sums of the rows above and below, then
        // remove the cell itself (it was counted by its own row's centre).
        buf.counts.resize(w * h, 0);
        let totals = &buf.row_totals;
        for_each_row(&mut buf.counts, w, |y, out| {
            let above = rows.prev[y];
            let below = rows.next[y];
            let centre = &totals[y * w..y * w + w];
            let cells = &self.cells[y * w..y * w + w];
            for x in 0..w {
                let mut n = centre[x] - u8::from(cells[x] == check_val);
                if let Some(ay) = above {
                    n += totals[ay * w + x];
                }
                if let Some(by) = below {
                    n += totals[by * w + x];
                }
                out[x] = n;
            }
        });
    }

    /// Returns `(alive_count, color_sum)` neighbour arrays, used by
    /// multi-state automata like Immigration and Rainbow.
    ///
    /// Convenience wrapper around [`Grid::neighbor_stats_into`].
    pub fn neighbor_stats_alive(&self, boundary: BoundaryMode) -> (Vec<u8>, Vec<u32>) {
        let mut buf = NeighborBuffers::default();
        self.neighbor_stats_into(boundary, &mut buf);
        (buf.counts, buf.sums)
    }

    /// Fill `buf.counts` with the number of live neighbours per cell and
    /// `buf.sums` with the sum of their states.
    pub fn neighbor_stats_into(&self, boundary: BoundaryMode, buf: &mut NeighborBuffers) {
        let (w, h) = (self.width, self.height);
        buf.counts.clear();
        buf.sums.clear();
        buf.row_live.clear();
        buf.row_values.clear();
        if w == 0 || h == 0 {
            return;
        }
        let cols = AxisWindow::new(w, boundary);
        let rows = AxisWindow::new(h, boundary);

        // Two additive channels: live-neighbour count and colour-value sum.
        buf.row_live.resize(w * h, 0);
        for_each_row_pair(&mut buf.row_live, &self.cells, w, |out, row| {
            for x in 0..w {
                out[x] = u32::from(row[x] > 0) + neighbour_sum(row, &cols, x, |v| u32::from(v > 0));
            }
        });
        buf.row_values.resize(w * h, 0);
        for_each_row_pair(&mut buf.row_values, &self.cells, w, |out, row| {
            for x in 0..w {
                out[x] = u32::from(row[x]) + neighbour_sum(row, &cols, x, u32::from);
            }
        });

        buf.counts.resize(w * h, 0);
        {
            let count_sums = &buf.row_live;
            for_each_row(&mut buf.counts, w, |y, out| {
                let above = rows.prev[y];
                let below = rows.next[y];
                let base = y * w;
                for x in 0..w {
                    let mut c = count_sums[base + x] - u32::from(self.cells[base + x] > 0);
                    if let Some(ay) = above {
                        c += count_sums[ay * w + x];
                    }
                    if let Some(by) = below {
                        c += count_sums[by * w + x];
                    }
                    out[x] = c as u8;
                }
            });
        }

        buf.sums.resize(w * h, 0);
        let value_sums = &buf.row_values;
        for_each_row(&mut buf.sums, w, |y, out| {
            let above = rows.prev[y];
            let below = rows.next[y];
            let base = y * w;
            for x in 0..w {
                let mut s = value_sums[base + x] - u32::from(self.cells[base + x]);
                if let Some(ay) = above {
                    s += value_sums[ay * w + x];
                }
                if let Some(by) = below {
                    s += value_sums[by * w + x];
                }
                out[x] = s;
            }
        });
    }
}

/// Reusable working buffers for neighbour counting.
///
/// Reusing one of these across simulation steps keeps the intermediate row sums
/// and the output arrays allocated instead of allocating several grid-sized
/// vectors per generation.
#[derive(Default)]
pub struct NeighborBuffers {
    /// Neighbour count for every cell (output).
    pub counts: Vec<u8>,
    /// Sum of live neighbour states for every cell (output of the stats pass).
    pub sums: Vec<u32>,
    /// Horizontal sums of cells equal to the requested value.
    row_totals: Vec<u8>,
    /// Horizontal sums of live neighbours.
    row_live: Vec<u32>,
    /// Horizontal sums of neighbour states.
    row_values: Vec<u32>,
}

/// Precomputed neighbour indices along one axis.
///
/// `prev[i]`/`next[i]` are `None` where the boundary discards the neighbour.
struct AxisWindow {
    prev: Vec<Option<usize>>,
    next: Vec<Option<usize>>,
}

impl AxisWindow {
    fn new(size: usize, mode: BoundaryMode) -> Self {
        let index = |i: i32| mode.resolve_axis(i, size);
        Self {
            prev: (0..size).map(|i| index(i as i32 - 1)).collect(),
            next: (0..size).map(|i| index(i as i32 + 1)).collect(),
        }
    }
}

#[inline]
fn neighbour_sum(row: &[u8], cols: &AxisWindow, x: usize, map: impl Fn(u8) -> u32) -> u32 {
    let mut s = 0;
    if let Some(l) = cols.prev[x] {
        s += map(row[l]);
    }
    if let Some(r) = cols.next[x] {
        s += map(row[r]);
    }
    s
}

/// Run `f(out_row, in_row)` for every row, in parallel on big grids.
fn for_each_row_pair<T, F>(out: &mut [T], cells: &[u8], w: usize, f: F)
where
    T: Send,
    F: Fn(&mut [T], &[u8]) + Sync + Send,
{
    if cells.len() >= PARALLEL_CELL_THRESHOLD {
        use rayon::prelude::*;
        out.par_chunks_mut(w)
            .zip(cells.par_chunks(w))
            .for_each(|(out_row, in_row)| f(out_row, in_row));
    } else {
        for (out_row, in_row) in out.chunks_mut(w).zip(cells.chunks(w)) {
            f(out_row, in_row);
        }
    }
}

/// Run `f(row_index, out_row)` for every row, in parallel on big grids.
fn for_each_row<T, F>(out: &mut [T], w: usize, f: F)
where
    T: Send,
    F: Fn(usize, &mut [T]) + Sync + Send,
{
    if out.len() >= PARALLEL_CELL_THRESHOLD {
        use rayon::prelude::*;
        out.par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, out_row)| f(y, out_row));
    } else {
        for (y, out_row) in out.chunks_mut(w).enumerate() {
            f(y, out_row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference implementation: resolves each of the eight neighbour
    /// coordinates individually. The optimised passes must agree with it.
    fn naive_counts(grid: &Grid, boundary: BoundaryMode, check_val: u8) -> Vec<u8> {
        let (w, h) = (grid.width, grid.height);
        let mut out = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut count = 0u8;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dy == 0 && dx == 0 {
                            continue;
                        }
                        if let Some((ny, nx)) = boundary.resolve(y as i32 + dy, x as i32 + dx, h, w)
                        {
                            if grid.cells[ny * w + nx] == check_val {
                                count += 1;
                            }
                        }
                    }
                }
                out[y * w + x] = count;
            }
        }
        out
    }

    fn naive_stats(grid: &Grid, boundary: BoundaryMode) -> (Vec<u8>, Vec<u32>) {
        let (w, h) = (grid.width, grid.height);
        let mut counts = vec![0u8; w * h];
        let mut sums = vec![0u32; w * h];
        for y in 0..h {
            for x in 0..w {
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dy == 0 && dx == 0 {
                            continue;
                        }
                        if let Some((ny, nx)) = boundary.resolve(y as i32 + dy, x as i32 + dx, h, w)
                        {
                            let value = grid.cells[ny * w + nx];
                            if value > 0 {
                                counts[y * w + x] += 1;
                                sums[y * w + x] += u32::from(value);
                            }
                        }
                    }
                }
            }
        }
        (counts, sums)
    }

    fn sample_grid(w: usize, h: usize) -> Grid {
        let mut grid = Grid::new(w, h);
        let mut state = 0x2545_f491_4f6c_dd1du64;
        for cell in grid.cells.iter_mut() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *cell = (state % 4) as u8;
        }
        grid
    }

    #[test]
    fn neighbor_counts_match_the_reference_implementation() {
        for mode in BoundaryMode::all().iter().copied() {
            for (w, h) in [(1, 1), (1, 5), (5, 1), (2, 2), (7, 3), (9, 9), (33, 17)] {
                let grid = sample_grid(w, h);
                for check_val in [0u8, 1, 2, 3] {
                    assert_eq!(
                        grid.neighbor_counts(mode, check_val),
                        naive_counts(&grid, mode, check_val),
                        "mode={mode:?} size={w}x{h} check_val={check_val}"
                    );
                }
            }
        }
    }

    #[test]
    fn neighbor_counts_match_the_reference_above_the_parallel_threshold() {
        let grid = sample_grid(300, 300);
        assert!(grid.cells.len() >= PARALLEL_CELL_THRESHOLD);
        for mode in BoundaryMode::all().iter().copied() {
            assert_eq!(
                grid.neighbor_counts(mode, 1),
                naive_counts(&grid, mode, 1),
                "parallel path, mode={mode:?}"
            );
        }
    }

    #[test]
    fn neighbor_stats_match_the_reference_implementation() {
        for mode in BoundaryMode::all().iter().copied() {
            for (w, h) in [(1, 1), (1, 4), (4, 1), (2, 3), (8, 8), (31, 13)] {
                let grid = sample_grid(w, h);
                assert_eq!(
                    grid.neighbor_stats_alive(mode),
                    naive_stats(&grid, mode),
                    "mode={mode:?} size={w}x{h}"
                );
            }
        }
        let big = sample_grid(300, 300);
        for mode in BoundaryMode::all().iter().copied() {
            assert_eq!(
                big.neighbor_stats_alive(mode),
                naive_stats(&big, mode),
                "parallel path, mode={mode:?}"
            );
        }
    }

    #[test]
    fn empty_dimensions_do_not_panic() {
        let grid = Grid::new(0, 0);
        assert!(grid.neighbor_counts(BoundaryMode::Wrap, 1).is_empty());
        assert_eq!(
            grid.neighbor_stats_alive(BoundaryMode::Wrap),
            (vec![], vec![])
        );
    }

    #[test]
    fn wrap_counts_eight_neighbours_of_a_single_live_cell() {
        // A one-cell grid under wrap sees itself eight times.
        let mut grid = Grid::new(1, 1);
        grid.set(0, 0, 1);
        assert_eq!(grid.neighbor_counts(BoundaryMode::Wrap, 1), vec![8]);
        assert_eq!(grid.neighbor_counts(BoundaryMode::Fixed, 1), vec![0]);
        assert_eq!(grid.neighbor_counts(BoundaryMode::Reflect, 1), vec![8]);
    }
}
