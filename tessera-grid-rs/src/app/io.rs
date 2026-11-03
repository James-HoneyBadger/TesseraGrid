//! State and media I/O: PNG, GIF, RLE, CSV and `.tessera` state files.

use super::*;

/// Current `.tessera` file schema version.
const STATE_VERSION: u32 = 3;

fn state_version() -> u32 {
    1
}

/// Grid snapshot written to `.tessera` state files.
///
/// `cells` is the automaton's *canonical* state: render-only markers such as
/// the ant are not stored, so a saved file can never feed an artefact back into
/// the simulation on load. `aux` carries the automaton state that lives outside
/// the grid.
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    #[serde(default = "state_version")]
    version: u32,
    automaton: String,
    generation: u64,
    width: usize,
    height: usize,
    cells: Vec<u8>,
    #[serde(default)]
    aux: Vec<u8>,
    #[serde(default)]
    settings: Option<crate::sim::EvolutionSettings>,
}

impl SavedState {
    fn capture(sim: &Simulation) -> Self {
        Self {
            version: STATE_VERSION,
            automaton: sim.mode().to_owned(),
            generation: sim.generation,
            width: sim.width(),
            height: sim.height(),
            cells: sim.state_cells(),
            aux: sim.aux_state(),
            settings: Some(sim.evolution_settings()),
        }
    }

    /// Reject dimensions the engine cannot honour.
    ///
    /// The file is arbitrary user input, so a hand-written (or corrupt) header
    /// claiming a billion rows must not become a billion-byte allocation. The
    /// cell count must also match the declared size, otherwise the grid would
    /// be silently left empty.
    fn validate(&self) -> Result<(), String> {
        if self.version > STATE_VERSION {
            return Err(format!(
                "file version {} is newer than this build understands ({STATE_VERSION})",
                self.version
            ));
        }
        if self.version == 0 {
            return Err("unsupported state version 0".to_owned());
        }
        if automata::Mode::from_name(&self.automaton).is_none() {
            return Err(format!("unknown mode '{}'", self.automaton));
        }
        if self.version >= 3 && self.settings.is_none() {
            return Err("state file is missing evolution settings".to_owned());
        }
        if !(crate::sim::MIN_GRID_SIDE..=crate::sim::MAX_GRID_SIDE).contains(&self.width)
            || !(crate::sim::MIN_GRID_SIDE..=crate::sim::MAX_GRID_SIDE).contains(&self.height)
        {
            return Err(format!(
                "grid size {}×{} is outside {}–{}",
                self.width,
                self.height,
                crate::sim::MIN_GRID_SIDE,
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

    fn simulation(&self) -> Result<Simulation, String> {
        self.validate()?;
        let mut sim = match &self.settings {
            Some(settings) => settings.build(&self.automaton, self.width, self.height)?,
            None => Simulation::new(
                &self.automaton,
                self.width,
                self.height,
                RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
                "Empty",
            ),
        };
        let aux = if self.version < 3 && self.aux.is_empty() {
            sim.aux_state()
        } else {
            self.aux.clone()
        };
        sim.restore_state(&self.cells, &aux, self.generation)?;
        Ok(sim)
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
        match export::export_png(&self.sim.display_grid(), cell_size, &path) {
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
            let state = SavedState::capture(&self.sim);
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
                        let mut sim = match state.simulation() {
                            Ok(sim) => sim,
                            Err(err) => {
                                self.set_status(format!("Parse error: {err}"));
                                return;
                            }
                        };
                        sim.detect_cycles = self.sim.detect_cycles;
                        self.selected_boundary = sim.boundary();
                        self.selected_mode = sim.mode().to_owned();
                        if self.selected_mode == "Custom Rules" {
                            self.custom_rule_str = sim.rule_string();
                            (self.custom_birth, self.custom_survival) =
                                LifeLike::try_parse_bs(&self.custom_rule_str)
                                    .expect("validated saved rule");
                        } else if self.selected_mode == "Elementary 1D" {
                            self.elementary_rule = sim.rule_string()[1..]
                                .parse()
                                .expect("validated Wolfram rule");
                        }
                        self.sim = sim;
                        self.iso_cache = None;
                        self.metrics_cache = None;
                        self.selected_pattern = "Empty".to_owned();
                        self.running = false;
                        self.run_n_pending = 0;
                        self.paint_state = self.paint_state.min(self.sim.max_state()).max(1);
                        self.viewport_offset = Vec2::ZERO;
                        self.selection = None;
                        self.selection_anchor = None;
                        let warning = if state.version < 3 {
                            "; legacy file: missing settings use wrap and default rules; missing auxiliary state uses defaults"
                        } else {
                            ""
                        };
                        self.set_status(format!("Loaded {}{warning}", path.display()));
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
    use super::{SavedState, STATE_VERSION};
    use crate::sim::MAX_GRID_SIDE;

    fn state(width: usize, height: usize, cells: Vec<u8>) -> SavedState {
        SavedState {
            version: STATE_VERSION,
            automaton: "Conway's Game of Life".to_owned(),
            generation: 0,
            width,
            height,
            cells,
            aux: Vec::new(),
            settings: Some(crate::sim::EvolutionSettings {
                boundary: crate::core::BoundaryMode::Wrap,
                rule: "B3/S23".to_owned(),
            }),
        }
    }

    #[test]
    fn an_ant_file_round_trips_without_carrying_the_marker() {
        use crate::core::BoundaryMode;
        use crate::sim::{RuleSpec, Simulation};

        let mut sim = Simulation::new(
            "Langton's Ant",
            8,
            8,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
            "Empty",
        );
        sim.run_steps(5, || false);

        let saved = SavedState::capture(&sim);
        assert!(
            saved.cells.iter().all(|&state| state <= 1),
            "the render marker must not reach the file"
        );
        assert_eq!(saved.aux.len(), 17, "ant position and heading are saved");

        let text = serde_json::to_string(&saved).expect("serialise");
        let loaded: SavedState = serde_json::from_str(&text).expect("deserialise");
        loaded.validate().expect("valid");

        let mut restored = loaded.simulation().expect("restore");
        sim.step();
        restored.step();
        assert_eq!(restored.generation, 6);
        assert_eq!(restored.state_cells(), sim.state_cells());
        assert_eq!(restored.aux_state(), sim.aux_state());
    }

    #[test]
    fn files_from_a_newer_build_are_rejected() {
        let mut newer = state(4, 3, vec![0; 12]);
        newer.version = STATE_VERSION + 1;
        let err = newer.validate().expect_err("newer files are refused");
        assert!(err.contains("newer"), "{err}");
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

    #[test]
    fn rules_and_boundaries_round_trip_future_evolution() {
        use crate::core::BoundaryMode;
        use crate::sim::{RuleSpec, Simulation};

        for boundary in BoundaryMode::all().iter().copied() {
            for mode in ["Custom Rules", "Elementary 1D"] {
                let mut original = Simulation::new(
                    mode,
                    11,
                    7,
                    RuleSpec::new(boundary, &[0, 3, 6], &[1, 2, 3]).with_elementary(30),
                    "Empty",
                );
                original.randomize(42);
                original.run_steps(4, || false);
                let text = serde_json::to_string(&SavedState::capture(&original)).unwrap();
                let saved: SavedState = serde_json::from_str(&text).unwrap();
                let mut loaded = saved.simulation().unwrap();
                assert_eq!(loaded.evolution_settings(), original.evolution_settings());
                assert_eq!(loaded.metrics(), original.metrics());
                for _ in 0..15 {
                    original.step();
                    loaded.step();
                    assert_eq!(loaded.state_cells(), original.state_cells());
                    assert_eq!(loaded.aux_state(), original.aux_state());
                }
            }
        }
    }

    #[test]
    fn corrupt_state_is_rejected_before_it_replaces_the_working_simulation() {
        let mut saved = state(4, 3, vec![0; 12]);
        saved.cells[0] = 2;
        assert!(saved.simulation().is_err());
        saved.cells[0] = 0;
        saved.settings = None;
        assert!(saved.simulation().is_err());
        saved.version = 2;
        assert!(
            saved.simulation().is_ok(),
            "legacy files use explicit documented defaults"
        );
        saved.automaton = "Unknown".to_owned();
        assert!(saved.simulation().is_err());
    }
}
