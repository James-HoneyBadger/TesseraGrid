//! Snapshot-based undo/redo history.

use std::collections::VecDeque;

/// Default byte budget for retained undo history (128 MiB).
///
/// A snapshot count alone is not enough once grids get large: 100 snapshots of
/// a 1000×1000 grid would pin 100 MB, on top of the timeline and frame budgets.
pub const DEFAULT_MAX_UNDO_BYTES: usize = 128 * 1024 * 1024;

/// A grid snapshot together with the generation it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cells: Vec<u8>,
    pub generation: u64,
    /// Opaque automaton state that lives outside the grid, such as Langton's
    /// Ant position and heading. Empty for automata without auxiliary state.
    pub aux: Vec<u8>,
}

impl Snapshot {
    pub fn new(cells: Vec<u8>, generation: u64) -> Self {
        Self {
            cells,
            generation,
            aux: Vec::new(),
        }
    }

    /// Attach automaton-specific auxiliary state.
    pub fn with_aux(mut self, aux: Vec<u8>) -> Self {
        self.aux = aux;
        self
    }

    pub(crate) fn byte_len(&self) -> usize {
        self.cells.len() + self.aux.len()
    }
}

/// Circular undo/redo stack for grid snapshots.
///
/// History is capped both by snapshot count and by total bytes, so a large grid
/// cannot pin an unbounded amount of memory.
pub struct UndoManager {
    undo_stack: VecDeque<Snapshot>,
    redo_stack: VecDeque<Snapshot>,
    max_history: usize,
    max_bytes: usize,
    undo_bytes: usize,
    redo_bytes: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self::with_budget(max_history, DEFAULT_MAX_UNDO_BYTES)
    }

    pub fn with_budget(max_history: usize, max_bytes: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: max_history.max(1),
            max_bytes: max_bytes.max(1),
            undo_bytes: 0,
            redo_bytes: 0,
        }
    }

    pub fn push(&mut self, snapshot: Snapshot) {
        self.undo_bytes += snapshot.byte_len();
        self.undo_stack.push_back(snapshot);
        trim(
            &mut self.undo_stack,
            &mut self.undo_bytes,
            self.max_history,
            self.max_bytes,
        );
        self.redo_stack.clear();
        self.redo_bytes = 0;
    }

    /// Returns the previous state. The caller supplies the current state so it
    /// can be replayed by [`Self::redo`].
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let prev = self.undo_stack.pop_back()?;
        self.undo_bytes -= prev.byte_len();
        self.redo_bytes += current.byte_len();
        self.redo_stack.push_back(current);
        trim(
            &mut self.redo_stack,
            &mut self.redo_bytes,
            self.max_history,
            self.max_bytes,
        );
        Some(prev)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo_stack.pop_back()?;
        self.redo_bytes -= next.byte_len();
        self.undo_bytes += current.byte_len();
        self.undo_stack.push_back(current);
        trim(
            &mut self.undo_stack,
            &mut self.undo_bytes,
            self.max_history,
            self.max_bytes,
        );
        Some(next)
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Bytes currently retained by the undo stack.
    pub fn undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    /// Number of snapshots currently retained by the undo stack.
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }
}

/// Drop the oldest entries until both the count and byte budgets are satisfied.
///
/// The newest snapshot is always kept, so undo never becomes a no-op purely
/// because a single grid snapshot exceeds the budget.
fn trim(stack: &mut VecDeque<Snapshot>, bytes: &mut usize, max_history: usize, max_bytes: usize) {
    while stack.len() > max_history || (*bytes > max_bytes && stack.len() > 1) {
        let Some(evicted) = stack.pop_front() else {
            break;
        };
        *bytes -= evicted.byte_len();
    }
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(cell: u8, generation: u64) -> Snapshot {
        Snapshot::new(vec![cell], generation)
    }

    #[test]
    fn undo_and_redo_round_trip() {
        let mut undo = UndoManager::new(4);
        undo.push(snapshot(1, 1));
        let restored = undo.undo(snapshot(2, 2)).expect("undo");
        assert_eq!(restored, snapshot(1, 1));
        assert!(undo.can_redo());

        let next = undo.redo(restored.clone()).expect("redo");
        assert_eq!(next, snapshot(2, 2));
        assert!(undo.can_undo());
    }

    #[test]
    fn push_clears_redo() {
        let mut undo = UndoManager::new(4);
        undo.push(snapshot(1, 1));
        undo.undo(snapshot(2, 2)).expect("undo");
        undo.push(snapshot(3, 3));
        assert!(!undo.can_redo());
    }

    #[test]
    fn history_is_capped() {
        let mut undo = UndoManager::new(2);
        undo.push(snapshot(1, 1));
        undo.push(snapshot(2, 2));
        undo.push(snapshot(3, 3));
        // Only the two most recent snapshots survive.
        assert_eq!(undo.undo(snapshot(4, 4)), Some(snapshot(3, 3)));
        assert_eq!(undo.undo(snapshot(3, 3)), Some(snapshot(2, 2)));
        assert!(!undo.can_undo());
    }

    #[test]
    fn byte_budget_evicts_oldest_snapshots() {
        // Each snapshot holds 10 bytes of cells, so three fit in a 30-byte budget.
        let mut undo = UndoManager::with_budget(100, 30);
        for generation in 0..10u64 {
            undo.push(Snapshot::new(vec![generation as u8; 10], generation));
        }
        assert_eq!(undo.undo_len(), 3);
        assert!(undo.undo_bytes() <= 30, "bytes = {}", undo.undo_bytes());
        // The newest snapshots are the ones that survived.
        assert_eq!(undo.undo(snapshot(0, 99)).map(|s| s.generation), Some(9));
    }

    #[test]
    fn byte_budget_keeps_the_newest_snapshot_even_when_it_is_oversized() {
        let mut undo = UndoManager::with_budget(100, 4);
        undo.push(Snapshot::new(vec![1; 64], 7));
        assert_eq!(undo.undo_len(), 1);
        let restored = undo.undo(snapshot(0, 8)).expect("newest snapshot is kept");
        assert_eq!(restored.generation, 7);
    }

    #[test]
    fn auxiliary_state_survives_a_round_trip() {
        let mut undo = UndoManager::new(4);
        undo.push(Snapshot::new(vec![1], 1).with_aux(vec![9, 8, 7]));
        let restored = undo.undo(snapshot(0, 2)).expect("undo");
        assert_eq!(restored.aux, vec![9, 8, 7]);
    }
}
