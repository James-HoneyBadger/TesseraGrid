//! State and media I/O: PNG, GIF, RLE, CSV and `.tessera` state files.

use super::*;

/// Grid snapshot written to `.tessera` state files.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    automaton: String,
    generation: u64,
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

impl SavedState {
    /// Reject dimensions the engine cannot honour.
    ///
    /// The file is arbitrary user input, so a hand-written (or corrupt) header
    /// claiming a billion rows must not become a billion-byte allocation. The
    /// cell count must also match the declared size, otherwise the grid would
    /// be silently left empty.
    fn validate(&self) -> Result<(), String> {
        if !(1..=crate::sim::MAX_GRID_SIDE).contains(&self.width)
            || !(1..=crate::sim::MAX_GRID_SIDE).contains(&self.height)
        {
            return Err(format!(
                "grid size {}×{} is outside 1–{}",
                self.width,
                self.height,
                crate::sim::MAX_GRID_SIDE
            ));
        }
        let expected = self.width * self.height;
        if self.cells.len() != expected {
            return Err(format!(
                "{} cells do not match a {}×{} grid",
                self.cells.len(),
                self.width,
                self.height
            ));
        }
        Ok(())
    }
}

impl TesseraGridApp {
    pub(super) fn export_png_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("tessera.png")
            .save_file()
        else {
            return;
        };
        let cell_size = (self.cell_size as u32).max(1);
        match export::export_png(self.sim.grid(), cell_size, &path) {
            Ok(()) => self.set_status(format!("Saved {}", path.display())),
            Err(err) => self.set_status(format!("Export failed: {err}")),
        }
    }

    pub(super) fn export_gif_dialog(&mut self) {
        if self.sim.frame_count() == 0 {
            self.set_status("Record frames before exporting a GIF");
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("GIF animation", &["gif"])
            .set_file_name("tessera.gif")
            .save_file()
        else {
            return;
        };
        let (gw, gh) = (self.sim.width(), self.sim.height());
        let cell_size = (self.cell_size as u32).clamp(1, 8);
        let delay = 1000 / self.speed.clamp(1, 30);
        let frame_count = self.sim.frame_count();
        let result = export::export_gif(self.sim.frames(), gw, gh, cell_size, delay, &path);
        match result {
            Ok(()) => self.set_status(format!("GIF saved ({frame_count} frames)")),
            Err(err) => self.set_status(format!("GIF failed: {err}")),
        }
    }

    pub(super) fn import_rle_from_clipboard(&mut self) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(text) => {
                let pattern = rle::parse(&text);
                if pattern.is_empty() {
                    self.set_status(if pattern.truncated {
                        "Clipboard: RLE too large to import"
                    } else {
                        "Clipboard: no valid RLE found"
                    });
                    return;
                }
                let (gw, gh) = (self.sim.width(), self.sim.height());
                let max_state = self.sim.max_state();
                self.push_undo();
                self.sim.edit_cells(|cells| {
                    for &(x, y, state) in &pattern.cells {
                        if x >= 0 && y >= 0 && (x as usize) < gw && (y as usize) < gh {
                            cells[y as usize * gw + x as usize] = state.min(max_state);
                        }
                    }
                });
                let rule = pattern.rule.as_deref().unwrap_or("none");
                if pattern.truncated {
                    self.set_status(format!(
                        "RLE imported: {}×{}, rule {rule} (truncated at {} cells)",
                        pattern.width,
                        pattern.height,
                        rle::MAX_CELLS
                    ));
                } else {
                    self.set_status(format!(
                        "RLE imported: {}×{}, rule {rule}",
                        pattern.width, pattern.height
                    ));
                }
            }
            Err(err) => self.set_status(format!("Clipboard error: {err}")),
        }
    }

    pub(super) fn copy_rle_to_clipboard(&mut self) {
        let encoded = rle::encode(self.sim.grid(), &self.sim.rule_string());
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(encoded)) {
            Ok(()) => self.set_status("RLE copied to clipboard"),
            Err(err) => self.set_status(format!("Clipboard error: {err}")),
        }
    }

    pub(super) fn export_stats_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("tessera_stats.csv")
            .save_file()
        else {
            return;
        };
        let history = self.sim.pop_history();
        let first_generation = self.sim.generation.saturating_sub(history.len() as u64);
        let mut out = String::from("generation,population\n");
        for (i, pop) in history.iter().enumerate() {
            out.push_str(&format!("{},{pop}\n", first_generation + i as u64 + 1));
        }
        match std::fs::write(&path, out) {
            Ok(()) => self.set_status(format!("Saved {}", path.display())),
            Err(err) => self.set_status(format!("CSV write failed: {err}")),
        }
    }

    pub(super) fn save_state(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tessera Grid state", &["tessera", "lifegrid"])
            .save_file()
        {
            let (w, h, cells) = {
                let g = self.sim.grid();
                (g.width, g.height, g.cells.clone())
            };
            let state = SavedState {
                automaton: self.selected_mode.clone(),
                generation: self.sim.generation,
                width: w,
                height: h,
                cells,
            };
            match serde_json::to_string_pretty(&state) {
                Ok(text) => match std::fs::write(&path, &text) {
                    Ok(()) => self.set_status(format!("Saved {}", path.display())),
                    Err(e) => self.set_status(format!("Write error: {e}")),
                },
                Err(e) => self.set_status(format!("Serialize error: {e}")),
            }
        }
    }

    pub(super) fn load_state(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tessera Grid state", &["tessera", "lifegrid"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<SavedState>(&text) {
                    Ok(state) => {
                        if let Err(err) = state.validate() {
                            self.set_status(format!("Parse error: {err}"));
                            return;
                        }
                        self.sim.rebuild(
                            &state.automaton,
                            state.width,
                            state.height,
                            RuleSpec::new(
                                self.selected_boundary,
                                &self.custom_birth,
                                &self.custom_survival,
                            ),
                            "",
                        );
                        self.sim.set_cells(&state.cells);
                        self.sim.generation = state.generation;
                        self.selected_mode = state.automaton;
                        self.selected_pattern = self
                            .sim
                            .available_patterns()
                            .first()
                            .copied()
                            .unwrap_or("")
                            .to_owned();
                        self.running = false;
                        self.viewport_offset = Vec2::ZERO;
                        self.selection = None;
                        self.selection_anchor = None;
                        self.set_status(format!("Loaded {}", path.display()));
                    }
                    Err(e) => self.set_status(format!("Parse error: {e}")),
                },
                Err(e) => self.set_status(format!("Read error: {e}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SavedState;
    use crate::sim::MAX_GRID_SIDE;

    fn state(width: usize, height: usize, cells: Vec<u8>) -> SavedState {
        SavedState {
            automaton: "Conway's Game of Life".to_owned(),
            generation: 0,
            width,
            height,
            cells,
        }
    }

    #[test]
    fn a_matching_file_is_accepted() {
        assert!(state(4, 3, vec![0; 12]).validate().is_ok());
    }

    #[test]
    fn a_hostile_header_is_rejected_before_allocating() {
        // This used to drive `Grid::new(1e9, 1e9)` and abort on allocation.
        let err = state(1_000_000_000, 1_000_000_000, Vec::new())
            .validate()
            .expect_err("oversized header must be rejected");
        assert!(err.contains("outside"), "{err}");

        let err = state(MAX_GRID_SIDE + 1, 1, vec![0; MAX_GRID_SIDE + 1])
            .validate()
            .expect_err("one past the limit must be rejected");
        assert!(err.contains("outside"), "{err}");
    }

    #[test]
    fn zero_sized_grids_are_rejected() {
        assert!(state(0, 10, Vec::new()).validate().is_err());
        assert!(state(10, 0, Vec::new()).validate().is_err());
    }

    #[test]
    fn a_cell_count_that_disagrees_with_the_size_is_rejected() {
        let err = state(4, 3, vec![1; 11])
            .validate()
            .expect_err("short cell list must be rejected");
        assert!(err.contains("do not match"), "{err}");
    }
}
