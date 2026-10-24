use super::*;

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
        match export::export_gif(self.sim.frames(), gw, gh, cell_size, delay, &path) {
            Ok(()) => self.set_status(format!("GIF saved ({} frames)", self.sim.frame_count())),
            Err(err) => self.set_status(format!("GIF failed: {err}")),
        }
    }

    pub(super) fn import_rle_from_clipboard(&mut self) {
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.get_text()) {
            Ok(text) => {
                let pattern = rle::parse(&text);
                if pattern.is_empty() {
                    self.set_status("Clipboard: no valid RLE found");
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
                self.set_status(format!(
                    "RLE imported: {}×{}, rule {}",
                    pattern.width,
                    pattern.height,
                    pattern.rule.as_deref().unwrap_or("none")
                ));
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
                        if state.cells.len() == self.sim.grid().cells.len() {
                            self.sim.set_cells(&state.cells);
                        }
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
