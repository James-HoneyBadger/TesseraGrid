//! Engine throughput benchmarks.
//!
//! Run with `cargo bench`. The harness is plain Rust so the crate needs no
//! benchmark dependency; the output is a per-operation timing table that makes
//! regressions in the hot loops visible.

use std::time::{Duration, Instant};

use tessera_grid::automata::{Automaton, HexagonalLife};
use tessera_grid::core::{BoundaryMode, Grid, NeighborBuffers};
use tessera_grid::sim::{RuleSpec, Simulation};

const SIZES: [(usize, usize); 2] = [(250, 250), (1000, 1000)];
const REPEATS: u32 = 10;

fn report(label: &str, size: (usize, usize), elapsed: Duration) {
    println!(
        "{label:<34} {:>4}x{:<4} {:>8.3} ms/iter",
        size.0,
        size.1,
        elapsed.as_secs_f64() * 1000.0 / f64::from(REPEATS)
    );
}

fn time(mut run: impl FnMut()) -> Duration {
    // Warm up so the first allocation does not dominate the measurement.
    run();
    let start = Instant::now();
    for _ in 0..REPEATS {
        run();
    }
    start.elapsed()
}

fn filled(width: usize, height: usize) -> Grid {
    let mut grid = Grid::new(width, height);
    let mut state = 0x2545_f491_4f6c_dd1du64;
    for cell in grid.cells.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *cell = u8::from(state.is_multiple_of(7));
    }
    grid
}

fn main() {
    for size in SIZES {
        let (w, h) = size;
        let grid = filled(w, h);

        let elapsed = time(|| {
            std::hint::black_box(grid.neighbor_counts(BoundaryMode::Wrap, 1));
        });
        report("neighbor_counts (allocating)", size, elapsed);

        let mut buffers = NeighborBuffers::default();
        let elapsed = time(|| {
            grid.neighbor_counts_into(BoundaryMode::Wrap, 1, &mut buffers);
            std::hint::black_box(&buffers.counts);
        });
        report("neighbor_counts_into (reused)", size, elapsed);

        let elapsed = time(|| {
            grid.neighbor_stats_alive(BoundaryMode::Wrap);
        });
        report("neighbor_stats_alive (allocating)", size, elapsed);

        let mut stats = NeighborBuffers::default();
        let elapsed = time(|| {
            grid.neighbor_stats_into(BoundaryMode::Wrap, &mut stats);
            std::hint::black_box(&stats.sums);
        });
        report("neighbor_stats_into (reused)", size, elapsed);

        let mut hex = HexagonalLife::new(w, h);
        let elapsed = time(|| hex.step());
        report("HexagonalLife step", size, elapsed);

        let mut life = Simulation::new(
            "Conway's Game of Life",
            w,
            h,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
            "Empty",
        );
        life.randomize(7);
        let elapsed = time(|| life.step());
        report("Simulation step (timeline on)", size, elapsed);

        life.record_timeline = false;
        let elapsed = time(|| life.step());
        report("Simulation step (timeline off)", size, elapsed);
        println!();
    }
}
