use super::*;

/// Clamp a drag between two raw grid coordinates to the grid and normalise it.
///
/// The pointer can legitimately be outside the grid (letterboxed grids, or
/// dragging past the edge). An unclamped selection used to produce
/// astronomically large region sizes, which panicked the app when the region was
/// copied, cut, or cleared.
///
/// Returns an inclusive `(x0, y0, x1, y1)` region with `x0 <= x1` and
/// `y0 <= y1`, or `None` for an empty grid.
pub(crate) fn clamp_selection(
    anchor: (i64, i64),
    point: (i64, i64),
    gw: usize,
    gh: usize,
) -> Option<(i64, i64, i64, i64)> {
    if gw == 0 || gh == 0 {
        return None;
    }
    let (max_x, max_y) = (gw as i64 - 1, gh as i64 - 1);
    let (ax, ay) = (anchor.0.clamp(0, max_x), anchor.1.clamp(0, max_y));
    let (px, py) = (point.0.clamp(0, max_x), point.1.clamp(0, max_y));
    Some((ax.min(px), ay.min(py), ax.max(px), ay.max(py)))
}

impl TesseraGridApp {
    /// Update the selection from a raw grid coordinate.
    ///
    /// The anchor stays at the first drag point; the region is always the box
    /// between it and the current pointer, clamped to the grid.
    pub(super) fn extend_selection(&mut self, x: i64, y: i64, gw: usize, gh: usize) {
        let anchor = *self.selection_anchor.get_or_insert((x, y));
        self.selection = clamp_selection(anchor, (x, y), gw, gh);
    }

    /// Inclusive, in-bounds selection bounds, or `None` when nothing is selected.
    ///
    /// Every consumer of [`TesseraGridApp::selection`] goes through this so a
    /// stale or out-of-range selection can never index off the grid.
    fn selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let (x0, y0, x1, y1) = self.selection?;
        let grid = self.sim.grid();
        if grid.width == 0 || grid.height == 0 {
            return None;
        }
        let x0 = x0.clamp(0, grid.width as i64 - 1) as usize;
        let y0 = y0.clamp(0, grid.height as i64 - 1) as usize;
        let x1 = x1.clamp(0, grid.width as i64 - 1) as usize;
        let y1 = y1.clamp(0, grid.height as i64 - 1) as usize;
        Some((x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)))
    }

    pub(super) fn copy_selection(&mut self) {
        let Some((x0c, y0c, x1c, y1c)) = self.selection_bounds() else {
            return;
        };
        let cw = x1c - x0c + 1;
        let ch = y1c - y0c + 1;
        let mut cells = vec![0u8; cw * ch];
        let grid = self.sim.grid();
        for dy in 0..ch {
            for dx in 0..cw {
                cells[dy * cw + dx] = grid.get(y0c + dy, x0c + dx);
            }
        }
        self.clipboard = Some((cells, cw, ch));
        self.set_status(format!("Copied {}×{} region", cw, ch));
    }

    pub(super) fn cut_selection(&mut self) {
        self.copy_selection();
        self.clear_selection_cells();
    }

    pub(super) fn paste_clipboard(&mut self) {
        if let Some((cells, cw, ch)) = self.clipboard.clone() {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width, g.height)
            };
            let (px, py) = match self.selection_bounds() {
                Some((x0, y0, _, _)) => (x0, y0),
                None => (gw.saturating_sub(cw) / 2, gh.saturating_sub(ch) / 2),
            };
            self.push_undo();
            self.sim.edit_cells(|grid_cells| {
                for dy in 0..ch {
                    for dx in 0..cw {
                        let gx = px + dx;
                        let gy = py + dy;
                        if gx < gw && gy < gh {
                            grid_cells[gy * gw + gx] = cells[dy * cw + dx];
                        }
                    }
                }
            });
            self.set_status(format!("Pasted {}×{} region", cw, ch));
        }
    }

    pub(super) fn clear_selection_cells(&mut self) {
        let Some((x0c, y0c, x1c, y1c)) = self.selection_bounds() else {
            return;
        };
        self.push_undo();
        let width = self.sim.width();
        self.sim.edit_cells(|grid_cells| {
            for y in y0c..=y1c {
                for x in x0c..=x1c {
                    grid_cells[y * width + x] = 0;
                }
            }
        });
        self.set_status("Selection cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_selection;

    #[test]
    fn a_drag_outside_the_grid_is_clamped() {
        // Both ends fully past the left/top edge used to produce a region of
        // ~1.8e19 cells, which panicked the copy/cut/clear paths.
        assert_eq!(
            clamp_selection((-30, -30), (-25, -25), 100, 100),
            Some((0, 0, 0, 0))
        );
    }

    #[test]
    fn a_drag_past_the_right_edge_is_clamped() {
        assert_eq!(
            clamp_selection((10, 10), (500, 500), 100, 80),
            Some((10, 10, 99, 79))
        );
    }

    #[test]
    fn a_reversed_drag_is_normalised() {
        assert_eq!(
            clamp_selection((30, 40), (10, 20), 100, 100),
            Some((10, 20, 30, 40))
        );
    }

    #[test]
    fn extremes_never_overflow() {
        let (x0, y0, x1, y1) =
            clamp_selection((i64::MIN, i64::MIN), (i64::MAX, i64::MAX), 4, 4).expect("region");
        assert_eq!((x0, y0, x1, y1), (0, 0, 3, 3));
    }

    #[test]
    fn an_empty_grid_has_no_selection() {
        assert_eq!(clamp_selection((0, 0), (5, 5), 0, 10), None);
        assert_eq!(clamp_selection((0, 0), (5, 5), 10, 0), None);
    }
}
