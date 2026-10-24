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
    pub fn neighbor_counts(&self, boundary: BoundaryMode, check_val: u8) -> Vec<u8> {
        let (w, h) = (self.width, self.height);
        let cells = &self.cells;
        self.map_cells(|i| {
            let (x, y) = (i % w, i / w);
            let mut c = 0u8;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dy == 0 && dx == 0 {
                        continue;
                    }
                    if let Some((ny, nx)) = boundary.resolve(y as i32 + dy, x as i32 + dx, h, w) {
                        if cells[ny * w + nx] == check_val {
                            c += 1;
                        }
                    }
                }
            }
            c
        })
    }

    /// Returns `(alive_count, color_sum)` neighbour arrays, used by
    /// multi-state automata like Immigration and Rainbow.
    pub fn neighbor_stats_alive(&self, boundary: BoundaryMode) -> (Vec<u8>, Vec<u32>) {
        let (w, h) = (self.width, self.height);
        let cells = &self.cells;
        let stats: Vec<(u8, u32)> = self.map_cells(|i| {
            let (x, y) = (i % w, i / w);
            let mut c = 0u8;
            let mut s = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dy == 0 && dx == 0 {
                        continue;
                    }
                    if let Some((ny, nx)) = boundary.resolve(y as i32 + dy, x as i32 + dx, h, w) {
                        let v = cells[ny * w + nx];
                        if v > 0 {
                            c += 1;
                            s += v as u32;
                        }
                    }
                }
            }
            (c, s)
        });
        stats.into_iter().unzip()
    }
}
