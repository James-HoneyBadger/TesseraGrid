mod canvas;
mod dialogs;
mod panels;
mod statusbar;
mod theme;
mod toolbar;

use std::time::{Duration, Instant};
pub(crate) use theme::{aging_color, cell_color, mode_accent_color};

use egui::{Color32, CornerRadius, Rect, Sense, Vec2};

use crate::automata::{self, LifeLike};
use crate::core::{AppConfig, BoundaryMode};
use crate::export;
use crate::patterns;
use crate::rle;
use crate::sim::{RuleSpec, Simulation};

/// Upper bound on simulation steps per rendered frame, so that high speeds do
/// not starve the UI thread.
const MAX_STEPS_PER_FRAME: u32 = 8;

/// Every keyboard shortcut, kept in one place for the help dialog.
const SHORTCUTS: &[(&str, &str)] = &[
    ("Space", "Play / pause"),
    ("S", "Step one generation"),
    ("R", "Reset to the selected pattern"),
    ("N", "Randomize with the current seed"),
    ("G", "Toggle grid lines"),
    ("F  /  Ctrl+0", "Zoom to fit"),
    ("V", "Toggle isometric 3-D view"),
    ("+  /  −", "Zoom in / out"),
    ("Mouse wheel", "Zoom at the cursor"),
    ("Middle-drag  /  arrows", "Pan the view"),
    ("Ctrl+Z", "Undo"),
    ("Ctrl+Y  /  Ctrl+Shift+Z", "Redo"),
    (
        "Ctrl+C  /  Ctrl+X  /  Ctrl+V",
        "Copy / cut / paste selection",
    ),
    ("Delete", "Clear selected cells"),
    ("Escape", "Deselect"),
    ("Ctrl+S  /  Ctrl+O", "Save / open state"),
    ("Ctrl+E", "Export PNG"),
    ("H  /  F1", "This help dialog"),
];

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// View mode (2-D flat vs isometric 3-D)
// ---------------------------------------------------------------------------

#[derive(PartialEq, Clone, Copy, Default)]
enum ViewMode {
    #[default]
    Flat,
    Isometric,
}

// ---------------------------------------------------------------------------
// Tool mode
// ---------------------------------------------------------------------------

#[derive(PartialEq, Clone, Copy, Default)]
enum ToolMode {
    #[default]
    Paint,
    Select,
}

// ---------------------------------------------------------------------------
// Saved state (serialised to .lifegrid JSON)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct SavedState {
    automaton: String,
    generation: u64,
    width: usize,
    height: usize,
    cells: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

pub struct LifeGridApp {
    // ── Simulation ──────────────────────────────────────────────────────────
    sim: Simulation,
    running: bool,
    last_step: Instant,

    // ── Settings ────────────────────────────────────────────────────────────
    speed: u32,
    cell_size: f32,
    show_grid: bool,
    dark_mode: bool,
    rounded_cells: bool,
    compact_mode: bool,

    // ── Mode / pattern selectors ─────────────────────────────────────────────
    selected_mode: String,
    selected_pattern: String,
    selected_boundary: BoundaryMode,
    custom_rule_str: String,
    custom_birth: Vec<u8>,
    custom_survival: Vec<u8>,
    seed_input: String,
    last_seed: u64,

    // ── Canvas ───────────────────────────────────────────────────────────────
    viewport_offset: Vec2,
    paint_state: u8,
    brush_size: u8,
    last_paint_cell: Option<(i64, i64)>,
    pending_fit: bool,
    drag_undo_pushed: bool,

    // ── Cell aging ───────────────────────────────────────────────────────────
    show_aging: bool,

    // ── Modals ───────────────────────────────────────────────────────────────
    show_run_n_dialog: bool,
    run_n_input: String,
    run_n_total: u64,
    run_n_pending: u64,
    show_resize_dialog: bool,
    resize_w_input: String,
    resize_h_input: String,
    resize_preserve: bool,
    resize_aspect: bool,

    // ── Rate tracking ────────────────────────────────────────────────────────
    last_frame_time: Instant,
    fps: f32,
    gen_rate: f32,
    gen_sample_time: Instant,
    gen_sample_generation: u64,

    // ── Status bar ───────────────────────────────────────────────────────────
    status_msg: Option<(String, Instant)>,

    // ── Canvas rect (updated every frame, used for zoom-to-fit) ──────────────
    canvas_rect: egui::Rect,
    hover_cell: Option<(i64, i64, u8)>,

    // ── Help & gallery modals ────────────────────────────────────────────────
    show_help: bool,
    show_gallery: bool,
    show_about: bool,
    gallery_filter: String,

    // ── Tool mode & selection ─────────────────────────────────────────────────
    tool_mode: ToolMode,
    selection: Option<(i64, i64, i64, i64)>, // (x0, y0, x1, y1) grid coords, inclusive
    selection_anchor: Option<(i64, i64)>,    // anchor cell during drag
    clipboard: Option<(Vec<u8>, usize, usize)>, // (cells, width, height)

    // ── Isometric 3-D view ─────────────────────────────────────────────────
    view_mode: ViewMode,
    iso_offset: Vec2, // manual pan in iso view
    iso_height: f32,  // block height multiplier (0.25–4.0)

    // ── Persisted layout ─────────────────────────────────────────────────────
    panel_width: f32,
    theme_key: Option<(bool, bool, Color32)>,
}

impl LifeGridApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (cfg, config_warning) = AppConfig::try_load();

        let boundary = BoundaryMode::all()
            .iter()
            .copied()
            .find(|m| m.as_str() == cfg.boundary)
            .unwrap_or_default();

        let mut sim = Simulation::new(
            &cfg.automaton_mode,
            cfg.grid_width,
            cfg.grid_height,
            RuleSpec::new(boundary, &cfg.custom_birth, &cfg.custom_survival),
            "",
        );

        let pattern = if sim.available_patterns().contains(&cfg.pattern.as_str()) {
            cfg.pattern.clone()
        } else {
            sim.available_patterns()
                .first()
                .copied()
                .unwrap_or("")
                .to_owned()
        };
        sim.load_pattern(&pattern);

        let custom_rule_str = {
            let b: String = cfg.custom_birth.iter().map(|n| n.to_string()).collect();
            let s: String = cfg.custom_survival.iter().map(|n| n.to_string()).collect();
            format!("B{}/S{}", b, s)
        };

        Self {
            sim,
            running: false,
            last_step: Instant::now(),

            speed: cfg.speed.clamp(1, 200),
            cell_size: cfg.cell_size.clamp(1.0, 64.0),
            show_grid: cfg.show_grid,
            dark_mode: cfg.dark_mode,
            rounded_cells: cfg.rounded_cells,
            compact_mode: cfg.compact_mode,

            selected_mode: cfg.automaton_mode,
            selected_pattern: pattern,
            selected_boundary: boundary,
            custom_rule_str,
            custom_birth: cfg.custom_birth,
            custom_survival: cfg.custom_survival,
            seed_input: "1".into(),
            last_seed: 1,

            viewport_offset: Vec2::ZERO,
            paint_state: cfg.paint_state.max(1),
            brush_size: cfg.brush_size.max(1),
            last_paint_cell: None,
            pending_fit: true,
            drag_undo_pushed: false,

            show_aging: cfg.show_aging,

            show_run_n_dialog: false,
            run_n_input: "100".into(),
            run_n_total: 0,
            run_n_pending: 0,
            show_resize_dialog: false,
            resize_w_input: "100".into(),
            resize_h_input: "100".into(),
            resize_preserve: false,
            resize_aspect: false,

            last_frame_time: Instant::now(),
            fps: 0.0,
            gen_rate: 0.0,
            gen_sample_time: Instant::now(),
            gen_sample_generation: 0,

            status_msg: config_warning.map(|msg| (msg, Instant::now())),

            canvas_rect: egui::Rect::ZERO,
            hover_cell: None,
            show_help: false,
            show_gallery: false,
            show_about: false,
            gallery_filter: String::new(),
            tool_mode: ToolMode::default(),
            selection: None,
            selection_anchor: None,
            clipboard: None,

            view_mode: if cfg.view_mode == "Isometric" {
                ViewMode::Isometric
            } else {
                ViewMode::Flat
            },
            iso_offset: Vec2::ZERO,
            iso_height: cfg.iso_height,
            panel_width: cfg.panel_width,
            theme_key: None,
        }
    }

    // ── Simulation helpers ───────────────────────────────────────────────────

    fn step_duration(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.speed.max(1) as f64)
    }

    fn do_step(&mut self) {
        self.sim.step();
    }

    fn randomize(&mut self) {
        let seed = self.seed_input.trim().parse::<u64>().unwrap_or_else(|_| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1)
        });
        self.push_undo();
        self.last_seed = seed;
        self.seed_input = seed.to_string();
        self.sim.randomize(seed);
        self.running = false;
        self.set_status(format!("Randomized (seed {seed})"));
    }

    fn do_reset(&mut self) {
        self.push_undo();
        self.sim.reset();
        self.running = false;
    }

    fn rebuild_automaton(&mut self) {
        let (w, h) = (self.sim.width(), self.sim.height());
        self.sim.rebuild(
            &self.selected_mode,
            w,
            h,
            RuleSpec::new(
                self.selected_boundary,
                &self.custom_birth,
                &self.custom_survival,
            ),
            &self.selected_pattern,
        );
        if !self
            .sim
            .available_patterns()
            .contains(&self.selected_pattern.as_str())
        {
            self.selected_pattern = self
                .sim
                .available_patterns()
                .first()
                .copied()
                .unwrap_or("")
                .to_owned();
            let pattern = self.selected_pattern.clone();
            self.sim.load_pattern(&pattern);
        }
        self.running = false;
        self.viewport_offset = Vec2::ZERO;
        self.selection = None;
        self.selection_anchor = None;
        self.paint_state = self.paint_state.min(self.sim.max_state()).max(1);
    }

    fn push_undo(&mut self) {
        self.sim.push_undo();
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), Instant::now()));
    }

    // ── Zoom to fit ──────────────────────────────────────────────────────────

    fn zoom_to_fit(&mut self) {
        let cw = self.canvas_rect.width();
        let ch = self.canvas_rect.height();
        if cw <= 0.0 || ch <= 0.0 {
            return;
        }
        let (gw, gh) = {
            let g = self.sim.grid();
            (g.width as f32, g.height as f32)
        };
        self.cell_size = (cw / gw).min(ch / gh).clamp(1.0, 64.0);
        let total_w = gw * self.cell_size;
        let total_h = gh * self.cell_size;
        self.viewport_offset = Vec2::new(
            ((total_w - cw) / 2.0).max(0.0),
            ((total_h - ch) / 2.0).max(0.0),
        );
    }

    fn iso_fit(&mut self) {
        let cw = self.canvas_rect.width();
        let ch = self.canvas_rect.height();
        if cw <= 0.0 || ch <= 0.0 {
            return;
        }
        let (gw, gh) = {
            let g = self.sim.grid();
            (g.width as f32, g.height as f32)
        };
        // Isometric screen extent: width ≈ (gw+gh)*cs/2, height ≈ (gw+gh)*cs/4 + height_margin
        let h_margin = 4.0 * self.iso_height; // state-units × cs/2 × iso_h, simplified
        let cs_w = cw * 2.0 / (gw + gh).max(1.0);
        let cs_h = ch / ((gw + gh) / 4.0 + h_margin).max(1.0);
        self.cell_size = cs_w.min(cs_h).clamp(1.0, 64.0);
        self.iso_offset = Vec2::ZERO;
    }

    fn export_png_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("lifegrid.png")
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

    fn export_gif_dialog(&mut self) {
        if self.sim.frame_count() == 0 {
            self.set_status("Record frames before exporting a GIF");
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("GIF animation", &["gif"])
            .set_file_name("lifegrid.gif")
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

    fn import_rle_from_clipboard(&mut self) {
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

    fn copy_rle_to_clipboard(&mut self) {
        let encoded = rle::encode(self.sim.grid(), &self.sim.rule_string());
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(encoded)) {
            Ok(()) => self.set_status("RLE copied to clipboard"),
            Err(err) => self.set_status(format!("Clipboard error: {err}")),
        }
    }

    fn export_stats_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("lifegrid_stats.csv")
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

    // ── Save / load grid state ───────────────────────────────────────────────

    fn save_state(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LifeGrid state", &["lifegrid"])
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

    fn load_state(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LifeGrid state", &["lifegrid"])
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

    // ── Selection actions ────────────────────────────────────────────────────

    fn copy_selection(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width as i64, g.height as i64)
            };
            let x0c = x0.max(0) as usize;
            let y0c = y0.max(0) as usize;
            let x1c = x1.min(gw - 1) as usize;
            let y1c = y1.min(gh - 1) as usize;
            if x0c > x1c || y0c > y1c {
                return;
            }
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
    }

    fn cut_selection(&mut self) {
        self.copy_selection();
        self.clear_selection_cells();
    }

    fn paste_clipboard(&mut self) {
        if let Some((cells, cw, ch)) = self.clipboard.clone() {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width, g.height)
            };
            let (px, py) = if let Some((x0, y0, _, _)) = self.selection {
                (x0.max(0) as usize, y0.max(0) as usize)
            } else {
                (gw.saturating_sub(cw) / 2, gh.saturating_sub(ch) / 2)
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

    fn clear_selection_cells(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let (gw, gh) = {
                let g = self.sim.grid();
                (g.width as i64, g.height as i64)
            };
            let x0c = x0.max(0) as usize;
            let y0c = y0.max(0) as usize;
            let x1c = x1.min(gw - 1) as usize;
            let y1c = y1.min(gh - 1) as usize;
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

    // ── Help modal ───────────────────────────────────────────────────────────

    fn save_config(&self) {
        let cfg = AppConfig {
            config_version: crate::core::config::CONFIG_VERSION,
            grid_width: self.sim.width(),
            grid_height: self.sim.height(),
            cell_size: self.cell_size,
            speed: self.speed,
            show_grid: self.show_grid,
            automaton_mode: self.selected_mode.clone(),
            pattern: self.selected_pattern.clone(),
            boundary: self.selected_boundary.as_str().to_owned(),
            custom_birth: self.custom_birth.clone(),
            custom_survival: self.custom_survival.clone(),
            dark_mode: self.dark_mode,
            panel_width: self.panel_width,
            rounded_cells: self.rounded_cells,
            paint_state: self.paint_state,
            show_aging: self.show_aging,
            compact_mode: self.compact_mode,
            view_mode: match self.view_mode {
                ViewMode::Flat => "Flat".to_owned(),
                ViewMode::Isometric => "Isometric".to_owned(),
            },
            tool_mode: match self.tool_mode {
                ToolMode::Paint => "Paint".to_owned(),
                ToolMode::Select => "Select".to_owned(),
            },
            brush_size: self.brush_size,
            iso_height: self.iso_height,
        };
        if let Err(err) = cfg.save() {
            eprintln!("LifeGrid: {err}");
        }
    }

    // ── Theme ────────────────────────────────────────────────────────────────

    // ── Keyboard shortcuts ───────────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Skip shortcuts when a text field is focused (e.g., custom rule input).
        // Never steal keys from a focused widget (text edit or focused button).
        if ctx.memory(|memory| memory.focused().is_some()) {
            return;
        }
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.running = !self.running;
                if self.running {
                    self.last_step = Instant::now();
                }
            }
            if i.key_pressed(egui::Key::S) {
                self.push_undo();
                self.do_step();
            }
            if i.key_pressed(egui::Key::N) {
                self.randomize();
            }
            if i.key_pressed(egui::Key::R) {
                self.do_reset();
                let pat = self.selected_pattern.clone();
                self.sim.load_pattern(&pat);
            }
            if i.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && self.sim.undo() {
                self.running = false;
            }
            if i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
                && self.sim.redo()
            {
                self.running = false;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_state();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                self.load_state();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) {
                self.export_png_dialog();
            }
            // Zoom
            let plus = i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals);
            let minus = i.key_pressed(egui::Key::Minus);
            if plus {
                self.cell_size = (self.cell_size + 1.0).min(64.0);
            }
            if minus {
                self.cell_size = (self.cell_size - 1.0).max(1.0);
            }

            // Zoom to fit
            if i.key_pressed(egui::Key::F) || (i.modifiers.ctrl && i.key_pressed(egui::Key::Num0)) {
                self.pending_fit = true;
            }

            // Arrow-key panning
            let pan = (self.cell_size * 5.0).max(10.0);
            if i.key_pressed(egui::Key::ArrowLeft) {
                self.viewport_offset.x -= pan;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                self.viewport_offset.x += pan;
            }
            if i.key_pressed(egui::Key::ArrowUp) {
                self.viewport_offset.y -= pan;
            }
            if i.key_pressed(egui::Key::ArrowDown) {
                self.viewport_offset.y += pan;
            }

            // Help
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::F1) {
                self.show_help = !self.show_help;
            }

            // Toggle 3-D view
            if i.key_pressed(egui::Key::V) {
                self.view_mode = match self.view_mode {
                    ViewMode::Flat => ViewMode::Isometric,
                    ViewMode::Isometric => ViewMode::Flat,
                };
                self.pending_fit = true;
            }

            // Selection shortcuts
            if i.key_pressed(egui::Key::Escape) {
                self.selection = None;
                self.selection_anchor = None;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) {
                self.copy_selection();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::X) {
                self.cut_selection();
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                self.paste_clipboard();
            }
            if i.key_pressed(egui::Key::Delete) {
                self.clear_selection_cells();
            }
        });
    }

    // ── UI panels ────────────────────────────────────────────────────────────

    // ── Modal dialogs ────────────────────────────────────────────────────────

    // ── Isometric 3-D canvas ─────────────────────────────────────────────────
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for LifeGridApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);

        // FPS tracking (exponential moving average)
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }
        self.last_frame_time = now;

        self.handle_keyboard(ctx);

        // Simulation tick: fixed timestep with catch-up so fast speeds are real.
        if self.running {
            let step_interval = self.step_duration();
            let now = Instant::now();
            let mut steps = 0;
            while now.duration_since(self.last_step) >= step_interval && steps < MAX_STEPS_PER_FRAME
            {
                self.sim.step();
                self.last_step += step_interval;
                steps += 1;
            }
            // Drop the backlog instead of spiralling if we hit the cap.
            if now.duration_since(self.last_step) >= step_interval {
                self.last_step = now;
            }
            ctx.request_repaint_after(step_interval);
        }

        // Run-N executes in short slices so the UI keeps repainting.
        if self.run_n_pending > 0 {
            let slice_start = Instant::now();
            let done = self.sim.run_steps(self.run_n_pending, || {
                slice_start.elapsed() >= Duration::from_millis(8)
            });
            self.run_n_pending -= done;
            if self.run_n_pending > 0 {
                ctx.request_repaint();
            } else {
                self.set_status(format!("Ran {} steps", self.run_n_total));
            }
        }

        // Generation-rate sampling (half-second window).
        let elapsed = now.duration_since(self.gen_sample_time).as_secs_f32();
        if elapsed >= 0.5 {
            let generations = self
                .sim
                .generation
                .saturating_sub(self.gen_sample_generation);
            self.gen_rate = generations as f32 / elapsed;
            self.gen_sample_time = now;
            self.gen_sample_generation = self.sim.generation;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Modal dialogs (rendered above all panels)
        self.ui_run_n_dialog(&ctx);
        self.ui_resize_dialog(&ctx);
        self.ui_help_modal(&ctx);
        self.ui_gallery(&ctx);
        self.ui_about(&ctx);

        let bar_fill = if self.dark_mode {
            Color32::from_rgb(24, 27, 34)
        } else {
            Color32::from_rgb(246, 248, 252)
        };
        let bar_stroke = if self.dark_mode {
            Color32::from_rgb(52, 58, 72)
        } else {
            Color32::from_rgb(208, 216, 228)
        };
        let bar_margin = if self.compact_mode {
            egui::Margin::symmetric(10, 6)
        } else {
            egui::Margin::symmetric(12, 8)
        };
        let bar_frame = egui::Frame::default()
            .fill(bar_fill)
            .stroke(egui::Stroke::new(1.0_f32, bar_stroke))
            .corner_radius(CornerRadius::same(0))
            .inner_margin(bar_margin);

        // Menu bar
        egui::Panel::top("menubar").frame(bar_frame).show(ui, |ui| {
            self.ui_menu_bar(ui);
        });

        // Toolbar
        egui::Panel::top("toolbar").frame(bar_frame).show(ui, |ui| {
            self.ui_toolbar(ui);
        });

        // Status bar
        egui::Panel::bottom("statusbar")
            .frame(bar_frame)
            .show(ui, |ui| {
                self.ui_statusbar(ui);
            });

        // Controls side panel
        let panel_min_width = if self.compact_mode { 160.0 } else { 180.0 };
        let panel_default_width = if self.compact_mode { 188.0 } else { 220.0 };
        let panel_target = self.panel_width.clamp(panel_min_width, 420.0);
        let panel = egui::Panel::left("controls")
            .resizable(true)
            .min_size(panel_min_width)
            .max_size(420.0)
            .default_size(panel_target.max(panel_default_width))
            .show(ui, |ui| {
                // Bound child widgets so they cannot stretch the panel.
                ui.set_max_width((panel_target - 20.0).max(120.0));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ui_controls(ui);
                });
            });
        self.panel_width = panel.response.rect.width().clamp(panel_min_width, 420.0);

        // Canvas
        egui::CentralPanel::default().show(ui, |ui| {
            self.ui_canvas(ui);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
}
