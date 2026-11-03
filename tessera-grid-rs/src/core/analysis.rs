//! Measurements for studying automata: state entropy, divergence between
//! neighbouring initial conditions, and cycle detection.
//!
//! Everything here works on plain cell buffers so it can be used from the
//! engine, the GUI and the CLI without depending on either.

use std::collections::HashMap;

/// Hash of a cell buffer, used to recognise repeated generations.
///
/// This is a word-at-a-time FNV-style mix, not a cryptographic hash: it only
/// has to be cheap and good enough that two different grids practically never
/// collide. Collisions would at worst report a shorter cycle.
pub fn state_hash(cells: &[u8]) -> u64 {
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let (words, remainder) = cells.as_chunks::<8>();
    for word in words {
        hash = (hash ^ u64::from_le_bytes(*word))
            .wrapping_mul(PRIME)
            .rotate_left(27);
    }
    for &byte in remainder {
        hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
    }
    hash
}

/// Shannon entropy of the state distribution, in bits per cell.
///
/// A uniform two-state grid scores 1.0; a grid where every cell holds the same
/// state scores 0.0.
pub fn shannon_entropy(cells: &[u8]) -> f64 {
    if cells.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for &cell in cells {
        counts[usize::from(cell)] += 1;
    }
    let total = cells.len() as f64;
    counts
        .iter()
        .filter(|&&count| count > 0)
        .map(|&count| {
            let p = count as f64 / total;
            -p * p.log2()
        })
        .sum()
}

/// Number of cells that differ between two buffers.
///
/// Runs are stopped at the shorter buffer, so grids of different sizes are
/// comparable up to their common prefix.
pub fn hamming_distance(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .filter(|(left, right)| left != right)
        .count()
}

/// Inclusive bounding box `(min_x, min_y, max_x, max_y)` of live cells.
pub fn bounding_box(
    cells: &[u8],
    width: usize,
    height: usize,
) -> Option<(usize, usize, usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }
    let mut found = false;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..height {
        let row = &cells[y * width..(y + 1) * width];
        let Some(first) = row.iter().position(|&cell| cell > 0) else {
            continue;
        };
        let last = row.iter().rposition(|&cell| cell > 0).unwrap_or(first);
        found = true;
        min_x = min_x.min(first);
        max_x = max_x.max(last);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    found.then_some((min_x, min_y, max_x, max_y))
}

/// A detected repeating generation sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cycle {
    /// Generations before the cycle starts.
    pub transient: u64,
    /// Cycle length in generations. `1` means the state is a still life.
    pub period: u64,
}

/// Detects when a simulation re-enters a state it has already visited.
///
/// Cellular automata are deterministic and finite, so every run must eventually
/// cycle. This keeps a bounded map from state hash to generation, which lets the
/// statistics panel and `--analyze` report the transient length and period of
/// small or settling systems.
pub struct CycleWatcher {
    seen: HashMap<u64, u64>,
    generation_of: u64,
    cycle: Option<Cycle>,
    capacity: usize,
}

impl CycleWatcher {
    /// `capacity` bounds the number of remembered generations (8 bytes of hash
    /// plus 8 bytes of generation each).
    pub fn new(capacity: usize) -> Self {
        Self {
            seen: HashMap::new(),
            generation_of: 0,
            cycle: None,
            capacity: capacity.max(1),
        }
    }

    /// Record the state at `generation`, returning the cycle once it is known.
    pub fn observe(&mut self, generation: u64, cells: &[u8]) -> Option<Cycle> {
        if self.cycle.is_some() {
            return self.cycle;
        }
        self.generation_of = generation;
        let hash = state_hash(cells);
        if let Some(&first) = self.seen.get(&hash) {
            self.cycle = Some(Cycle {
                transient: first,
                period: generation - first,
            });
            return self.cycle;
        }
        if self.seen.len() >= self.capacity {
            // Forget everything rather than grow without bound: a later repeat
            // is still detected, only the reported transient becomes local.
            self.seen.clear();
        }
        self.seen.insert(hash, generation);
        None
    }

    /// Cycle found so far, if any.
    pub fn cycle(&self) -> Option<Cycle> {
        self.cycle
    }

    /// Generation of the most recent observation.
    pub fn last_generation(&self) -> u64 {
        self.generation_of
    }

    /// Forget the history, keeping the configured capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
        self.cycle = None;
        self.generation_of = 0;
    }
}

/// Snapshot of the measurements reported for one generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub generation: u64,
    pub population: u64,
    pub density: f64,
    /// Shannon entropy of the state distribution, in bits per cell.
    pub entropy: f64,
    /// Inclusive `(min_x, min_y, max_x, max_y)` of live cells.
    pub bbox: Option<(usize, usize, usize, usize)>,
    pub cycle: Option<Cycle>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_is_zero_for_a_uniform_grid() {
        assert_eq!(shannon_entropy(&[0; 64]), 0.0);
        assert_eq!(shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn entropy_is_one_bit_for_an_even_split() {
        let mut cells = [0u8; 100];
        cells[..50].fill(1);
        assert!((shannon_entropy(&cells) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn entropy_grows_with_the_number_of_states() {
        let four_states = [0u8, 1, 2, 3];
        assert!((shannon_entropy(&four_states) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn hashing_distinguishes_states_and_is_stable() {
        let a = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut b = a;
        assert_eq!(state_hash(&a), state_hash(&a));
        b[8] = 10;
        assert_ne!(state_hash(&a), state_hash(&b));
        // Lengths that are not a multiple of eight still hash.
        assert_ne!(state_hash(&a[..5]), state_hash(&a[..4]));
    }

    #[test]
    fn hamming_distance_counts_differences() {
        assert_eq!(hamming_distance(&[1, 0, 1], &[1, 0, 1]), 0);
        assert_eq!(hamming_distance(&[1, 0, 1], &[1, 1, 0]), 2);
        assert_eq!(hamming_distance(&[1, 1], &[1]), 0);
    }

    #[test]
    fn bounding_box_covers_live_cells_only() {
        let mut cells = vec![0u8; 5 * 4];
        cells[7] = 1; // row 1, column 2
        cells[3 * 5 + 4] = 1;
        assert_eq!(bounding_box(&cells, 5, 4), Some((2, 1, 4, 3)));
        assert_eq!(bounding_box(&[0u8; 20], 5, 4), None);
        assert_eq!(bounding_box(&[], 0, 4), None);
    }

    #[test]
    fn a_still_life_is_a_period_one_cycle() {
        let mut watcher = CycleWatcher::new(100);
        let block = [1u8, 1, 0, 0, 1, 1, 0, 0];
        assert_eq!(watcher.observe(0, &block), None);
        assert_eq!(
            watcher.observe(1, &block),
            Some(Cycle {
                transient: 0,
                period: 1
            })
        );
    }

    #[test]
    fn a_blinker_reports_a_two_generation_cycle() {
        let mut watcher = CycleWatcher::new(100);
        let horizontal = [1u8, 1, 1, 0];
        let vertical = [1u8, 0, 1, 0];
        assert_eq!(watcher.observe(0, &[9, 9, 9, 9]), None, "startup state");
        assert_eq!(watcher.observe(1, &horizontal), None);
        assert_eq!(watcher.observe(2, &vertical), None);
        assert_eq!(
            watcher.observe(3, &horizontal),
            Some(Cycle {
                transient: 1,
                period: 2
            })
        );
        // Once found, the answer is stable.
        assert_eq!(watcher.cycle(), watcher.observe(4, &vertical));
    }

    #[test]
    fn the_watcher_forgets_history_instead_of_growing() {
        let mut watcher = CycleWatcher::new(4);
        for generation in 0..1000u64 {
            watcher.observe(generation, &[generation as u8]);
        }
        assert!(watcher.cycle().is_none());
        assert_eq!(watcher.last_generation(), 999);

        watcher.reset();
        assert_eq!(watcher.cycle(), None);
        assert_eq!(watcher.last_generation(), 0);
    }
}
