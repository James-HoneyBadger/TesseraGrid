//! Measurements for studying automata: state entropy, divergence between
//! neighbouring initial conditions, and cycle detection.
//!
//! Everything here works on plain cell buffers so it can be used from the
//! engine, the GUI and the CLI without depending on either.

use std::collections::HashMap;
use std::io::{self, Write};

use super::BoundaryMode;

/// Hash of a cell buffer, used to recognise repeated generations.
///
/// This is a non-cryptographic mix. Cycle detection verifies state equality
/// after a hash match.
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
/// Unmatched cells count as differences, rather than silently ignoring them.
pub fn hamming_distance(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b)
        .filter(|(left, right)| left != right)
        .count()
        + a.len().abs_diff(b.len())
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
    /// First retained generation on the cycle. This is an upper bound on the
    /// transient if observation started late or earlier states were discarded.
    pub transient: u64,
    /// Cycle length in generations. `1` means the state is a still life.
    pub period: u64,
}

/// Detects when a simulation re-enters a state it has already visited.
///
/// Cellular automata are deterministic and finite, so every run must eventually
/// cycle. Hash matches are verified against retained states. Retention is
/// bounded by both entry count and state bytes; no repeat within that window
/// does not prove that the system is aperiodic.
pub struct CycleWatcher {
    seen: HashMap<u64, Vec<Observation>>,
    generation_of: Option<u64>,
    cycle: Option<Cycle>,
    capacity: usize,
    max_bytes: usize,
    bytes: usize,
    entries: usize,
}

struct Observation {
    generation: u64,
    cells: Box<[u8]>,
    aux: Box<[u8]>,
}

impl CycleWatcher {
    /// Retain at most `capacity` generations and 32 MiB of state payloads.
    pub fn new(capacity: usize) -> Self {
        Self::with_budget(capacity, 32 * 1024 * 1024)
    }

    /// Bound entry overhead and retained cell/auxiliary-state payloads.
    pub fn with_budget(capacity: usize, max_bytes: usize) -> Self {
        Self {
            seen: HashMap::new(),
            generation_of: None,
            cycle: None,
            capacity: capacity.max(1),
            max_bytes,
            bytes: 0,
            entries: 0,
        }
    }

    /// Record the state at `generation`, returning the cycle once it is known.
    pub fn observe(&mut self, generation: u64, cells: &[u8]) -> Option<Cycle> {
        self.observe_with_aux(generation, cells, &[])
    }

    /// Include non-cell state that affects subsequent evolution.
    pub fn observe_with_aux(&mut self, generation: u64, cells: &[u8], aux: &[u8]) -> Option<Cycle> {
        let hash = state_hash(cells) ^ state_hash(aux).rotate_left(17);
        self.observe_hashed(generation, cells, aux, hash)
    }

    fn observe_hashed(
        &mut self,
        generation: u64,
        cells: &[u8],
        aux: &[u8],
        hash: u64,
    ) -> Option<Cycle> {
        if self.generation_of.is_some_and(|last| generation < last) {
            self.reset();
        }
        if self.generation_of == Some(generation) {
            return self.cycle;
        }
        self.generation_of = Some(generation);
        if self.cycle.is_some() {
            return self.cycle;
        }
        let previous = self.seen.get(&hash).and_then(|observations| {
            observations
                .iter()
                .find(|entry| entry.cells.as_ref() == cells && entry.aux.as_ref() == aux)
        });
        if let Some(first) = previous {
            self.cycle = Some(Cycle {
                transient: first.generation,
                period: generation - first.generation,
            });
            return self.cycle;
        }
        let bytes = cells.len() + aux.len();
        if self.entries >= self.capacity || self.bytes.saturating_add(bytes) > self.max_bytes {
            self.seen.clear();
            self.entries = 0;
            self.bytes = 0;
        }
        if bytes <= self.max_bytes {
            self.seen.entry(hash).or_default().push(Observation {
                generation,
                cells: cells.into(),
                aux: aux.into(),
            });
            self.entries += 1;
            self.bytes += bytes;
        }
        None
    }

    /// Cycle found so far, if any.
    pub fn cycle(&self) -> Option<Cycle> {
        self.cycle
    }

    /// Generation of the most recent observation.
    pub fn last_generation(&self) -> u64 {
        self.generation_of.unwrap_or(0)
    }

    /// Forget the history, keeping the configured capacity.
    pub fn reset(&mut self) {
        self.seen.clear();
        self.cycle = None;
        self.generation_of = None;
        self.bytes = 0;
        self.entries = 0;
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

/// Reproducible run description attached to every metrics CSV row.
pub struct AnalysisMetadata {
    pub mode: String,
    pub rule: String,
    pub boundary: BoundaryMode,
    pub width: usize,
    pub height: usize,
    pub initialization: String,
    pub seed: Option<u64>,
}

impl AnalysisMetadata {
    /// Common schema for time series, sweeps and replication results.
    pub fn write_header(writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "generation,population,density,entropy,bounds_width,bounds_height,cycle_period,cycle_first_seen,cycle_status,mode,rule,boundary,grid_width,grid_height,initialization,seed,version,metric_scope")
    }

    pub fn write_row(&self, writer: &mut impl Write, metrics: Metrics) -> io::Result<()> {
        let (width, height) = metrics
            .bbox
            .map_or((0, 0), |(x0, y0, x1, y1)| (x1 - x0 + 1, y1 - y0 + 1));
        let (period, transient, status) = match metrics.cycle {
            Some(cycle) => (
                cycle.period.to_string(),
                cycle.transient.to_string(),
                "detected",
            ),
            None => (String::new(), String::new(), "not_observed"),
        };
        let quote = |text: &str| format!("\"{}\"", text.replace('"', "\"\""));
        writeln!(
            writer,
            "{},{},{:.9},{:.9},{width},{height},{period},{transient},{status},{},{},{},{},{},{},{},{},active_cells",
            metrics.generation,
            metrics.population,
            metrics.density,
            metrics.entropy,
            quote(&self.mode),
            quote(&self.rule),
            self.boundary.as_str(),
            self.width,
            self.height,
            quote(&self.initialization),
            self.seed.map_or_else(String::new, |seed| seed.to_string()),
            env!("CARGO_PKG_VERSION"),
        )
    }
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
        assert_eq!(hamming_distance(&[1, 1], &[1]), 1);
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

    #[test]
    fn hashes_are_only_candidates_and_auxiliary_state_matters() {
        let mut watcher = CycleWatcher::new(10);
        assert_eq!(watcher.observe_hashed(0, &[1], &[0], 42), None);
        assert_eq!(watcher.observe_hashed(1, &[2], &[0], 42), None);
        assert_eq!(watcher.observe_hashed(2, &[1], &[1], 42), None);
        assert_eq!(watcher.observe_hashed(3, &[1], &[0], 42).unwrap().period, 3);
    }

    #[test]
    fn duplicate_and_backwards_generations_do_not_create_invalid_periods() {
        let mut watcher = CycleWatcher::new(10);
        assert_eq!(watcher.observe(5, &[1]), None);
        assert_eq!(watcher.observe(5, &[1]), None);
        assert_eq!(watcher.observe(3, &[1]), None);
        assert_eq!(watcher.observe(4, &[1]).unwrap().period, 1);
    }

    #[test]
    fn retained_state_respects_the_byte_budget() {
        let mut watcher = CycleWatcher::with_budget(100, 4);
        for generation in 0..10 {
            watcher.observe(generation, &[generation as u8; 3]);
            assert!(watcher.bytes <= 4);
        }
        assert_eq!(watcher.entries, 1);
        assert_eq!(watcher.observe(10, &[0; 5]), None);
        assert_eq!(watcher.bytes, 0);
    }

    #[test]
    fn csv_escapes_text_and_preserves_unknown_cycle_status() {
        let metadata = AnalysisMetadata {
            mode: "a,\"mode\"".to_owned(),
            rule: "W30".to_owned(),
            boundary: BoundaryMode::Fixed,
            width: 8,
            height: 8,
            initialization: "Single Cell".to_owned(),
            seed: None,
        };
        let mut csv = Vec::new();
        AnalysisMetadata::write_header(&mut csv).unwrap();
        metadata
            .write_row(
                &mut csv,
                Metrics {
                    generation: 0,
                    population: 1,
                    density: 0.125,
                    entropy: 0.5,
                    bbox: Some((4, 0, 4, 0)),
                    cycle: None,
                },
            )
            .unwrap();
        let csv = String::from_utf8(csv).unwrap();
        assert!(csv.contains(",,,not_observed,\"a,\"\"mode\"\"\",\"W30\",Fixed,8,8,"));
        assert_eq!(csv.lines().count(), 2);
    }
}
