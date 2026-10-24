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
        let h = height as i32;
        let w = width as i32;
        match self {
            Self::Wrap => Some((y.rem_euclid(h) as usize, x.rem_euclid(w) as usize)),
            Self::Fixed => {
                if y >= 0 && y < h && x >= 0 && x < w {
                    Some((y as usize, x as usize))
                } else {
                    None
                }
            }
            Self::Reflect => Some((reflect(y, h), reflect(x, w))),
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
