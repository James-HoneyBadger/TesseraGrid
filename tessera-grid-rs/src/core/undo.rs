use std::collections::VecDeque;

/// A grid snapshot together with the generation it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub cells: Vec<u8>,
    pub generation: u64,
}

/// Circular undo/redo stack for grid snapshots.
pub struct UndoManager {
    undo_stack: VecDeque<Snapshot>,
    redo_stack: VecDeque<Snapshot>,
    max_history: usize,
}

impl UndoManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: max_history.max(1),
        }
    }

    pub fn push(&mut self, snapshot: Snapshot) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(snapshot);
        self.redo_stack.clear();
    }

    /// Returns the previous state. The caller supplies the current state so it
    /// can be replayed by [`Self::redo`].
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let prev = self.undo_stack.pop_back()?;
        if self.redo_stack.len() >= self.max_history {
            self.redo_stack.pop_front();
        }
        self.redo_stack.push_back(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo_stack.pop_back()?;
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(current);
        Some(next)
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
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
        Snapshot {
            cells: vec![cell],
            generation,
        }
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
}
