//! Edge handling for neighbour lookups.

/// Available boundary conditions for cellular automata.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum BoundaryMode {
    /// Toroidal – edges wrap to the opposite side.
    #[default]
    Wrap,
    /// Fixed (dead) – cells outside the grid are treated as 0.
    Fixed,
    /// Reflect – edges mirror the interior.
    Reflect,
}

impl BoundaryMode {
    /// Resolve a potentially out-of-bounds (y, x) coordinate.
    ///
    /// Returns `Some((y, x))` if the coordinate maps to a valid cell, or
    /// `None` if it falls outside and the boundary mode discards it.
    #[inline]
    pub fn resolve(self, y: i32, x: i32, height: usize, width: usize) -> Option<(usize, usize)> {
        Some((self.resolve_axis(y, height)?, self.resolve_axis(x, width)?))
    }

    /// Resolve a single axis coordinate.
    ///
    /// Returns `Some(index)` when the coordinate maps into `0..size`, or `None`
    /// when it falls outside and the mode discards it. Both axes of
    /// [`Self::resolve`] share this mapping, so neighbour loops can precompute
    /// the index tables once instead of resolving per cell.
    #[inline]
    pub fn resolve_axis(self, i: i32, size: usize) -> Option<usize> {
        let size_i = size as i32;
        match self {
            Self::Wrap => Some(i.rem_euclid(size_i) as usize),
            Self::Fixed => (i >= 0 && i < size_i).then_some(i as usize),
            Self::Reflect => Some(reflect(i, size_i)),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wrap => "Wrap",
            Self::Fixed => "Fixed",
            Self::Reflect => "Reflect",
        }
    }

    pub fn all() -> &'static [BoundaryMode] {
        &[Self::Wrap, Self::Fixed, Self::Reflect]
    }
}

fn reflect(i: i32, size: i32) -> usize {
    if i < 0 {
        (-i - 1).min(size - 1) as usize
    } else if i >= size {
        (2 * size - i - 1).max(0) as usize
    } else {
        i as usize
    }
}
