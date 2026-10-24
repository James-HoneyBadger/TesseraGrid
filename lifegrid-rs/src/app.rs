use std::time::{Duration, Instant};

use egui::{Color32, CornerRadius, Rect, Sense, Vec2};

use crate::automata::{self, Automaton, LifeLike};
use crate::core::{AppConfig, BoundaryMode, UndoManager};
use crate::export;

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

fn cell_color(state: u8, dark: bool) -> Color32 {
    if dark {
        match state {
            0 => Color32::from_rgb(14, 14, 22),
            1 => Color32::from_rgb(220, 220, 255),
            2 => Color32::from_rgb(255, 80, 80),
            3 => Color32::from_rgb(255, 160, 0),
            4 => Color32::from_rgb(240, 240, 0),
            5 => Color32::from_rgb(0, 220, 100),
            6 => Color32::from_rgb(60, 140, 255),
            7 => Color32::from_rgb(200, 80, 255),
            8 => Color32::from_rgb(140, 140, 160),
            _ => Color32::from_rgb(100, 100, 120),
        }
    } else {
        match state {
            0 => Color32::WHITE,
            1 => Color32::BLACK,
            2 => Color32::from_rgb(200, 40, 40),
            3 => Color32::from_rgb(210, 110, 0),
            4 => Color32::from_rgb(180, 180, 0),
            5 => Color32::from_rgb(0, 150, 60),
            6 => Color32::from_rgb(0, 60, 200),
            7 => Color32::from_rgb(120, 0, 200),
            8 => Color32::from_rgb(80, 80, 80),
            _ => Color32::from_rgb(120, 120, 120),
        }
    }
}

fn aging_color(base: Color32, age: u32) -> Color32 {
    let t = (age as f32 / 200.0).min(1.0);
    let warm = Color32::from_rgb(255, 180, 40);
    let r = (base.r() as f32 * (1.0 - t) + warm.r() as f32 * t) as u8;
    let g = (base.g() as f32 * (1.0 - t) + warm.g() as f32 * t) as u8;
    let b = (base.b() as f32 * (1.0 - t) + warm.b() as f32 * t) as u8;
    Color32::from_rgb(r, g, b)
}

fn mode_accent_color(mode: &str) -> Color32 {
    match mode {
        "Conway's Game of Life" => Color32::from_rgb(0, 180, 220),
        "High Life" => Color32::from_rgb(0, 210, 120),
        "Hexagonal Life" => Color32::from_rgb(160, 80, 240),
        "Immigration Game" => Color32::from_rgb(240, 160, 0),
        "Rainbow Game" => Color32::from_rgb(240, 60, 120),
        "Langton's Ant" => Color32::from_rgb(255, 120, 0),
        "Wireworld" => Color32::from_rgb(255, 210, 0),
        "Brian's Brain" => Color32::from_rgb(80, 140, 255),
        "Generations" => Color32::from_rgb(0, 220, 180),
        "Custom Rules" => Color32::from_rgb(180, 180, 180),
        _ => Color32::GRAY,
    }
}

// ---------------------------------------------------------------------------
// RLE clipboard import
// ---------------------------------------------------------------------------

/// Parse an RLE-encoded pattern string into (dx, dy) offsets from (0, 0).
/// Supports the standard 2-state RLE format used by Golly and LifeWiki.
fn parse_rle(rle: &str) -> Vec<(i32, i32)> {
    let mut pts = Vec::new();
    // Concatenate all non-comment, non-header lines into one string.
    let data: String = rle
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with('x'))
        .collect::<Vec<_>>()
        .join("");
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    let mut count_str = String::new();
    for ch in data.chars() {
        match ch {
            '0'..='9' => count_str.push(ch),
            'b' | '.' => {
                let n = count_str.parse::<i32>().unwrap_or(1);
                x += n;
                count_str.clear();
            }
            'o' | 'A'..='X' => {
                let n = count_str.parse::<i32>().unwrap_or(1);
                for dx in 0..n {
                    pts.push((x + dx, y));
                }
                x += n;
                count_str.clear();
            }
            '$' => {
                let n = count_str.parse::<i32>().unwrap_or(1);
                y += n;
                x = 0;
                count_str.clear();
            }
            '!' => break,
            ' ' | '\t' | '\r' | '\n' => {}
            _ => {
                count_str.clear();
            }
        }
    }
    pts
}

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
    automaton: Box<dyn Automaton>,
    undo: UndoManager,
    running: bool,
    generation: u64,
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

    // ── Canvas ───────────────────────────────────────────────────────────────
    viewport_offset: Vec2,
    paint_state: u8,
    drag_undo_pushed: bool,

    // ── Cell aging ───────────────────────────────────────────────────────────
    age_grid: Vec<u32>,
    show_aging: bool,

    // ── GIF recording ────────────────────────────────────────────────────────
    record_frames: bool,
    frame_buffer: Vec<Vec<u8>>,

    // ── Statistics ───────────────────────────────────────────────────────────
    pop_history: Vec<usize>,

    // ── Modals ───────────────────────────────────────────────────────────────
    show_run_n_dialog: bool,
    run_n_input: String,
    show_resize_dialog: bool,
    resize_w_input: String,
    resize_h_input: String,

    // ── FPS tracking ─────────────────────────────────────────────────────────
    last_frame_time: Instant,
    fps: f32,

    // ── Status bar ───────────────────────────────────────────────────────────
    status_msg: Option<(String, Instant)>,

    // ── Canvas rect (updated every frame, used for zoom-to-fit) ──────────────
    canvas_rect: egui::Rect,

    // ── Help modal ───────────────────────────────────────────────────────────
    show_help: bool,

    // ── Tool mode & selection ─────────────────────────────────────────────────
    tool_mode: ToolMode,
    selection: Option<(i64, i64, i64, i64)>, // (x0, y0, x1, y1) grid coords, inclusive
    selection_anchor: Option<(i64, i64)>,    // anchor cell during drag
    clipboard: Option<(Vec<u8>, usize, usize)>, // (cells, width, height)

    // ── Isometric 3-D view ─────────────────────────────────────────────────
    view_mode: ViewMode,
    iso_offset: Vec2, // manual pan in iso view
    iso_height: f32,  // block height multiplier (0.25–4.0)
}

impl LifeGridApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let cfg = AppConfig::load();

        let boundary = BoundaryMode::all()
            .iter()
            .copied()
            .find(|m| m.as_str() == cfg.boundary)
            .unwrap_or_default();

        let mut automaton = Self::make_auto(
            &cfg.automaton_mode,
            cfg.grid_width,
            cfg.grid_height,
            boundary,
            &cfg.custom_birth,
            &cfg.custom_survival,
        );

        let pattern = if automaton
            .available_patterns()
            .contains(&cfg.pattern.as_str())
        {
            cfg.pattern.clone()
        } else {
            automaton
                .available_patterns()
                .first()
                .copied()
                .unwrap_or("")
                .to_owned()
        };
        automaton.load_pattern(&pattern);

        let n = automaton.get_grid().cells.len();

        let custom_rule_str = {
            let b: String = cfg.custom_birth.iter().map(|n| n.to_string()).collect();
            let s: String = cfg.custom_survival.iter().map(|n| n.to_string()).collect();
            format!("B{}/S{}", b, s)
        };

        Self {
            automaton,
            undo: UndoManager::new(100),
            running: false,
            generation: 0,
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

            viewport_offset: Vec2::ZERO,
            paint_state: cfg.paint_state.max(1),
            drag_undo_pushed: false,

            age_grid: vec![0u32; n],
            show_aging: cfg.show_aging,

            record_frames: false,
            frame_buffer: Vec::new(),

            pop_history: Vec::new(),

            show_run_n_dialog: false,
            run_n_input: "100".into(),
            show_resize_dialog: false,
            resize_w_input: "100".into(),
            resize_h_input: "100".into(),

            last_frame_time: Instant::now(),
            fps: 0.0,

            status_msg: None,

            canvas_rect: egui::Rect::ZERO,
            show_help: false,
            tool_mode: ToolMode::default(),
            selection: None,
            selection_anchor: None,
            clipboard: None,

            view_mode: ViewMode::default(),
            iso_offset: Vec2::ZERO,
            iso_height: 1.0,
        }
    }

    // ── Factory ─────────────────────────────────────────────────────────────

    fn make_auto(
        mode: &str,
        w: usize,
        h: usize,
        boundary: BoundaryMode,
        birth: &[u8],
        survival: &[u8],
    ) -> Box<dyn Automaton> {
        let mut auto: Box<dyn Automaton> = if mode == "Custom Rules" {
            Box::new(LifeLike::new(w, h, birth.to_vec(), survival.to_vec()))
        } else {
            automata::make_automaton(mode, w, h)
        };
        auto.set_boundary(boundary);
        auto
    }

    // ── Simulation helpers ───────────────────────────────────────────────────

    fn step_duration(&self) -> Duration {
        Duration::from_millis(1000 / self.speed.max(1) as u64)
    }

    fn do_step(&mut self) {
        self.automaton.step();
        self.generation += 1;

        // Snapshot cells before any further borrows.
        let (n, pop, cells) = {
            let g = self.automaton.get_grid();
            (g.cells.len(), g.population(), g.cells.clone())
        };

        // Update age grid.
        if self.age_grid.len() != n {
            self.age_grid = vec![0u32; n];
        }
        for (i, &state) in cells.iter().enumerate() {
            self.age_grid[i] = if state != 0 {
                self.age_grid[i].saturating_add(1)
            } else {
                0
            };
        }

        self.pop_history.push(pop);
        if self.pop_history.len() > 500 {
            self.pop_history.remove(0);
        }

        // GIF frame recording.
        if self.record_frames {
            self.frame_buffer.push(cells);
            if self.frame_buffer.len() > 500 {
                self.frame_buffer.remove(0);
            }
        }
    }

    fn do_reset(&mut self) {
        self.automaton.reset();
        self.generation = 0;
        self.pop_history.clear();
        self.running = false;
        let n = self.automaton.get_grid().cells.len();
        self.age_grid = vec![0u32; n];
        self.frame_buffer.clear();
    }

    fn rebuild_automaton(&mut self) {
        let (w, h) = {
            let g = self.automaton.get_grid();
            (g.width, g.height)
        };
        let mut new_auto = Self::make_auto(
            &self.selected_mode,
            w,
            h,
            self.selected_boundary,
            &self.custom_birth,
            &self.custom_survival,
        );
        let patterns = new_auto.available_patterns();
        if !patterns.contains(&self.selected_pattern.as_str()) {
            self.selected_pattern = patterns.first().copied().unwrap_or("").to_owned();
        }
        new_auto.load_pattern(&self.selected_pattern);
        let n = new_auto.get_grid().cells.len();
        self.automaton = new_auto;
        self.generation = 0;
        self.pop_history.clear();
        self.running = false;
        self.undo.clear();
        self.age_grid = vec![0u32; n];
        self.frame_buffer.clear();
        self.viewport_offset = Vec2::ZERO;
    }

    fn push_undo(&mut self) {
        let snapshot = self.automaton.get_grid().cells.clone();
        self.undo.push(snapshot);
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
            let g = self.automaton.get_grid();
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
            let g = self.automaton.get_grid();
            (g.width as f32, g.height as f32)
        };
        // Isometric screen extent: width ≈ (gw+gh)*cs/2, height ≈ (gw+gh)*cs/4 + height_margin
        let h_margin = 4.0 * self.iso_height; // state-units × cs/2 × iso_h, simplified
        let cs_w = cw * 2.0 / (gw + gh).max(1.0);
        let cs_h = ch / ((gw + gh) / 4.0 + h_margin).max(1.0);
        self.cell_size = cs_w.min(cs_h).clamp(1.0, 64.0);
        self.iso_offset = Vec2::ZERO;
    }

    // ── RLE export ───────────────────────────────────────────────────────────

    fn grid_to_rle(&self) -> String {
        let grid = self.automaton.get_grid();
        let w = grid.width;
        let h = grid.height;
        // Find bounding box of live cells.
        let mut x0 = w;
        let mut y0 = h;
        let mut x1 = 0usize;
        let mut y1 = 0usize;
        let mut found = false;
        for y in 0..h {
            for x in 0..w {
                if grid.get(y, x) != 0 {
                    if !found {
                        x0 = x;
                        y0 = y;
                        x1 = x;
                        y1 = y;
                        found = true;
                    } else {
                        x0 = x0.min(x);
                        x1 = x1.max(x);
                        y0 = y0.min(y);
                        y1 = y1.max(y);
                    }
                }
            }
        }
        if !found {
            return "x = 0, y = 0, rule = B3/S23\n!".to_owned();
        }
        let rw = x1 - x0 + 1;
        let rh = y1 - y0 + 1;
        let mut out = format!("x = {}, y = {}, rule = B3/S23\n", rw, rh);
        for y in y0..=y1 {
            let mut runs: Vec<(char, u32)> = Vec::new();
            for x in x0..=x1 {
                let ch = if grid.get(y, x) != 0 { 'o' } else { 'b' };
                if runs.last().map(|r| r.0) == Some(ch) {
                    runs.last_mut().unwrap().1 += 1;
                } else {
                    runs.push((ch, 1));
                }
            }
            // Omit trailing dead cells.
            while matches!(runs.last(), Some(('b', _))) {
                runs.pop();
            }
            for (ch, count) in &runs {
                if *count > 1 {
                    out.push_str(&count.to_string());
                }
                out.push(*ch);
            }
            if y < y1 {
                out.push('$');
            }
        }
        out.push('!');
        out
    }

    // ── Save / load grid state ───────────────────────────────────────────────

    fn save_state(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("LifeGrid state", &["lifegrid"])
            .save_file()
        {
            let (w, h, cells) = {
                let g = self.automaton.get_grid();
                (g.width, g.height, g.cells.clone())
            };
            let state = SavedState {
                automaton: self.selected_mode.clone(),
                generation: self.generation,
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
                        let mut new_auto = Self::make_auto(
                            &state.automaton,
                            state.width,
                            state.height,
                            self.selected_boundary,
                            &self.custom_birth,
                            &self.custom_survival,
                        );
                        let grid = new_auto.get_grid_mut();
                        if grid.cells.len() == state.cells.len() {
                            grid.cells = state.cells;
                        }
                        let n = new_auto.get_grid().cells.len();
                        self.automaton = new_auto;
                        self.selected_mode = state.automaton;
                        self.generation = state.generation;
                        self.pop_history.clear();
                        self.running = false;
                        self.undo.clear();
                        self.age_grid = vec![0u32; n];
                        self.frame_buffer.clear();
                        self.viewport_offset = Vec2::ZERO;
                        self.selection = None;
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
                let g = self.automaton.get_grid();
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
            let grid = self.automaton.get_grid();
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
                let g = self.automaton.get_grid();
                (g.width, g.height)
            };
            let (px, py) = if let Some((x0, y0, _, _)) = self.selection {
                (x0.max(0) as usize, y0.max(0) as usize)
            } else {
                (gw.saturating_sub(cw) / 2, gh.saturating_sub(ch) / 2)
            };
            self.push_undo();
            let grid = self.automaton.get_grid_mut();
            for dy in 0..ch {
                for dx in 0..cw {
                    let gx = px + dx;
                    let gy = py + dy;
                    if gx < gw && gy < gh {
                        grid.set(gy, gx, cells[dy * cw + dx]);
                    }
                }
            }
            self.set_status(format!("Pasted {}×{} region", cw, ch));
        }
    }

    fn clear_selection_cells(&mut self) {
        if let Some((x0, y0, x1, y1)) = self.selection {
            let (gw, gh) = {
                let g = self.automaton.get_grid();
                (g.width as i64, g.height as i64)
            };
            let x0c = x0.max(0) as usize;
            let y0c = y0.max(0) as usize;
            let x1c = x1.min(gw - 1) as usize;
            let y1c = y1.min(gh - 1) as usize;
            self.push_undo();
            let grid = self.automaton.get_grid_mut();
            for y in y0c..=y1c {
                for x in x0c..=x1c {
                    grid.set(y, x, 0);
                }
            }
            self.set_status("Selection cleared");
        }
    }

    // ── Help modal ───────────────────────────────────────────────────────────

    fn ui_help_modal(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        let mut open = true;
        egui::Window::new("\u{2328}  Keyboard Shortcuts")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        let entries: &[(&str, &str)] = &[
                            ("Space", "Play / Pause"),
                            ("S", "Step one generation"),
                            ("R", "Reset pattern"),
                            ("G", "Toggle grid lines"),
                            ("F", "Zoom to fit grid"),
                            ("V", "Toggle isometric 3-D view"),
                            ("+  /  −", "Zoom in / out"),
                            ("Mouse wheel", "Zoom (centred on cursor)"),
                            ("Middle-click drag", "Pan viewport"),
                            ("Arrow keys", "Pan viewport"),
                            ("Ctrl+Z", "Undo"),
                            ("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
                            ("Ctrl+C", "Copy selection"),
                            ("Ctrl+X", "Cut selection"),
                            ("Ctrl+V", "Paste clipboard"),
                            ("Delete", "Clear selected cells"),
                            ("Escape", "Deselect"),
                            ("H  /  F1", "This help dialog"),
                        ];
                        for (key, desc) in entries {
                            ui.label(egui::RichText::new(*key).monospace().strong());
                            ui.label(*desc);
                            ui.end_row();
                        }
                    });
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    self.show_help = false;
                }
            });
        if !open {
            self.show_help = false;
        }
    }

    fn save_config(&self) {
        let grid = self.automaton.get_grid();
        let cfg = AppConfig {
            grid_width: grid.width,
            grid_height: grid.height,
            cell_size: self.cell_size,
            speed: self.speed,
            show_grid: self.show_grid,
            dark_mode: self.dark_mode,
            rounded_cells: self.rounded_cells,
            panel_width: 220.0,
            automaton_mode: self.selected_mode.clone(),
            pattern: self.selected_pattern.clone(),
            boundary: self.selected_boundary.as_str().to_owned(),
            custom_birth: self.custom_birth.clone(),
            custom_survival: self.custom_survival.clone(),
            paint_state: self.paint_state,
            show_aging: self.show_aging,
            compact_mode: self.compact_mode,
        };
        cfg.save();
    }

    // ── Theme ────────────────────────────────────────────────────────────────

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });

        let accent = mode_accent_color(&self.selected_mode);
        let mut style = (*ctx.global_style()).clone();
        {
            let visuals = &mut style.visuals;
            if self.dark_mode {
                visuals.window_fill = Color32::from_rgb(22, 24, 30);
                visuals.panel_fill = Color32::from_rgb(18, 20, 26);
                visuals.faint_bg_color = Color32::from_rgb(30, 34, 42);
                visuals.extreme_bg_color = Color32::from_rgb(12, 14, 20);
            } else {
                visuals.window_fill = Color32::from_rgb(248, 249, 252);
                visuals.panel_fill = Color32::from_rgb(241, 244, 249);
                visuals.faint_bg_color = Color32::from_rgb(231, 236, 243);
                visuals.extreme_bg_color = Color32::from_rgb(220, 226, 236);
            }

            visuals.widgets.active.corner_radius = CornerRadius::same(6);
            visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
            visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
            visuals.widgets.open.corner_radius = CornerRadius::same(6);

            visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, accent);
            visuals.selection.bg_fill = accent.linear_multiply(0.25);
            visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent);
        }

        if self.compact_mode {
            style.spacing.item_spacing = Vec2::new(6.0, 6.0);
            style.spacing.button_padding = Vec2::new(8.0, 5.0);
            style.spacing.interact_size.y = 22.0;
            style.spacing.slider_width = 110.0;
        } else {
            style.spacing.item_spacing = Vec2::new(8.0, 8.0);
            style.spacing.button_padding = Vec2::new(10.0, 6.0);
            style.spacing.interact_size.y = 26.0;
            style.spacing.slider_width = 130.0;
        }
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.indent = 14.0;
        style.visuals.window_corner_radius = CornerRadius::same(10);
        style.visuals.menu_corner_radius = CornerRadius::same(8);
        ctx.set_global_style(style);
    }

    // ── Keyboard shortcuts ───────────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Skip shortcuts when a text field is focused (e.g., custom rule input).
        if ctx.egui_wants_keyboard_input() {
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
            if i.key_pressed(egui::Key::R) {
                self.do_reset();
                let pat = self.selected_pattern.clone();
                self.automaton.load_pattern(&pat);
            }
            if i.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                let current = self.automaton.get_grid().cells.clone();
                if let Some(prev) = self.undo.undo(current) {
                    self.automaton.get_grid_mut().cells = prev;
                    if self.generation > 0 {
                        self.generation -= 1;
                    }
                }
            }
            if i.modifiers.ctrl
                && (i.key_pressed(egui::Key::Y)
                    || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))
            {
                let current = self.automaton.get_grid().cells.clone();
                if let Some(next) = self.undo.redo(current) {
                    self.automaton.get_grid_mut().cells = next;
                    self.generation += 1;
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                // Export PNG via keyboard shortcut (non-blocking open below)
                self.status_msg = Some(("Use Export PNG… button".into(), Instant::now()));
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
            if i.key_pressed(egui::Key::F) {
                match self.view_mode {
                    ViewMode::Flat => self.zoom_to_fit(),
                    ViewMode::Isometric => self.iso_fit(),
                }
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
                if self.view_mode == ViewMode::Isometric {
                    self.iso_fit();
                }
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

    fn ui_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let accent = mode_accent_color(&self.selected_mode);

            // Coloured mode dot + title
            ui.colored_label(accent, "●");
            ui.label(egui::RichText::new("LifeGrid").strong().size(16.0));
            ui.separator();

            // Play / Pause
            let play_label = if self.running { "Pause" } else { "Play" };
            if ui
                .button(play_label)
                .on_hover_text(if self.running {
                    "Pause (Space)"
                } else {
                    "Play (Space)"
                })
                .clicked()
            {
                self.running = !self.running;
                if self.running {
                    self.last_step = Instant::now();
                }
            }
            // Step
            if ui.button("Step").on_hover_text("Step (S)").clicked() {
                self.push_undo();
                self.do_step();
            }
            // Reset
            if ui.button("Reset").on_hover_text("Reset (R)").clicked() {
                self.do_reset();
                let pat = self.selected_pattern.clone();
                self.automaton.load_pattern(&pat);
            }

            ui.separator();

            // Undo / Redo
            let can_undo = self.undo.can_undo();
            let can_redo = self.undo.can_redo();
            if ui
                .add_enabled(can_undo, egui::Button::new("Undo"))
                .on_hover_text("Undo (Ctrl+Z)")
                .clicked()
            {
                let cur = self.automaton.get_grid().cells.clone();
                if let Some(prev) = self.undo.undo(cur) {
                    self.automaton.get_grid_mut().cells = prev;
                    if self.generation > 0 {
                        self.generation -= 1;
                    }
                }
            }
            if ui
                .add_enabled(can_redo, egui::Button::new("Redo"))
                .on_hover_text("Redo (Ctrl+Y)")
                .clicked()
            {
                let cur = self.automaton.get_grid().cells.clone();
                if let Some(next) = self.undo.redo(cur) {
                    self.automaton.get_grid_mut().cells = next;
                    self.generation += 1;
                }
            }

            ui.separator();

            // Run N steps
            if ui
                .button("Run N…")
                .on_hover_text("Run a fixed number of steps")
                .clicked()
            {
                self.show_run_n_dialog = true;
            }

            ui.separator();

            // Theme toggle
            let theme_label = if self.dark_mode { "Light" } else { "Dark" };
            if ui
                .button(theme_label)
                .on_hover_text("Toggle dark/light theme")
                .clicked()
            {
                self.dark_mode = !self.dark_mode;
            }

            let compact_active = self.compact_mode;
            if ui
                .selectable_label(compact_active, "Compact")
                .on_hover_text("Tighter spacing for smaller screens")
                .clicked()
            {
                self.compact_mode = !self.compact_mode;
            }

            ui.separator();

            // Zoom to fit
            if ui
                .button("Fit")
                .on_hover_text("Zoom to fit grid (F)")
                .clicked()
            {
                self.zoom_to_fit();
            }

            ui.separator();

            // Tool mode
            let paint_active = self.tool_mode == ToolMode::Paint;
            let sel_active = self.tool_mode == ToolMode::Select;
            if ui
                .selectable_label(paint_active, "Paint")
                .on_hover_text("Paint tool — draw cells (default)")
                .clicked()
            {
                self.tool_mode = ToolMode::Paint;
                self.selection = None;
                self.selection_anchor = None;
            }
            if ui
                .selectable_label(sel_active, "Select")
                .on_hover_text("Select tool — drag to select a region (copy/paste/clear)")
                .clicked()
            {
                self.tool_mode = ToolMode::Select;
            }

            ui.separator();

            // 3-D / isometric view toggle
            let iso_active = self.view_mode == ViewMode::Isometric;
            if ui
                .selectable_label(iso_active, "Iso 3D")
                .on_hover_text("Toggle isometric 3-D view (V)")
                .clicked()
            {
                self.view_mode = if iso_active {
                    ViewMode::Flat
                } else {
                    ViewMode::Isometric
                };
                if self.view_mode == ViewMode::Isometric {
                    self.iso_fit();
                }
            }

            ui.separator();

            // Help
            if ui
                .button("Help")
                .on_hover_text("Keyboard shortcuts (H / F1)")
                .clicked()
            {
                self.show_help = !self.show_help;
            }

            // Version — right-aligned
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
            });
        });
    }

    fn ui_statusbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let (gw, gh, pop) = {
                let g = self.automaton.get_grid();
                (g.width, g.height, g.population())
            };
            let density = pop as f32 / (gw * gh).max(1) as f32 * 100.0;

            ui.label(egui::RichText::new(format!("Gen {}", self.generation)).strong());
            ui.separator();
            ui.label(format!("Pop {}", pop));
            ui.separator();
            ui.label(format!("Density {:.1}%", density));
            ui.separator();
            ui.label(format!("FPS: {:.0}", self.fps));

            // Timed status message
            if let Some((ref msg, ts)) = self.status_msg.clone() {
                if ts.elapsed() < Duration::from_secs(4) {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(100, 220, 100), msg);
                } else {
                    self.status_msg = None;
                }
            }
        });
    }

    fn ui_controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = if self.compact_mode {
            Vec2::new(6.0, 6.0)
        } else {
            Vec2::new(8.0, 8.0)
        };

        let card_fill = if self.dark_mode {
            Color32::from_rgb(24, 27, 34)
        } else {
            Color32::from_rgb(246, 248, 252)
        };
        let card_stroke = if self.dark_mode {
            Color32::from_rgb(52, 58, 72)
        } else {
            Color32::from_rgb(208, 216, 228)
        };
        let card_margin = if self.compact_mode {
            egui::Margin::same(8)
        } else {
            egui::Margin::same(10)
        };
        let button_h = if self.compact_mode { 24.0 } else { 28.0 };

        let section_frame = egui::Frame::group(ui.style())
            .fill(card_fill)
            .stroke(egui::Stroke::new(1.0_f32, card_stroke))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(card_margin);

        // ── Selection panel (visible only in Select mode) ────────────────────
        if self.tool_mode == ToolMode::Select {
            section_frame.show(ui, |ui| {
                egui::CollapsingHeader::new("Selection")
                    .default_open(true)
                    .show(ui, |ui| {
                        if let Some((x0, y0, x1, y1)) = self.selection {
                            let sw = x1 - x0 + 1;
                            let sh = y1 - y0 + 1;
                            ui.label(format!("{}×{}  at  ({}, {})", sw, sh, x0, y0));
                            ui.horizontal(|ui| {
                                let w = (ui.available_width() - 12.0) / 3.0;
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Copy"))
                                    .on_hover_text("Ctrl+C")
                                    .clicked()
                                {
                                    self.copy_selection();
                                }
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Cut"))
                                    .on_hover_text("Ctrl+X")
                                    .clicked()
                                {
                                    self.cut_selection();
                                }
                                if ui
                                    .add_sized([w, button_h], egui::Button::new("Clear"))
                                    .on_hover_text("Delete")
                                    .clicked()
                                {
                                    self.clear_selection_cells();
                                }
                            });
                            if ui
                                .add_sized(
                                    [ui.available_width(), button_h],
                                    egui::Button::new("Deselect"),
                                )
                                .on_hover_text("Escape")
                                .clicked()
                            {
                                self.selection = None;
                            }
                        } else {
                            ui.weak("Drag on canvas to select a region.");
                        }
                        if let Some((_, cw, ch)) = &self.clipboard {
                            ui.separator();
                            ui.label(format!("Clipboard: {}×{}", cw, ch));
                            if ui
                                .add_sized(
                                    [ui.available_width(), button_h],
                                    egui::Button::new("Paste"),
                                )
                                .on_hover_text("Ctrl+V")
                                .clicked()
                            {
                                self.paste_clipboard();
                            }
                        }
                    });
            });
            ui.add_space(4.0);
        }

        // ── Automaton section ────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Automaton")
                .default_open(true)
                .show(ui, |ui| {
                    // Mode
                    ui.label("Mode:");
                    let old_mode = self.selected_mode.clone();
                    egui::ComboBox::from_id_salt("mode_combo")
                        .selected_text(&self.selected_mode)
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &m in automata::ALL_MODES {
                                ui.selectable_value(&mut self.selected_mode, m.to_owned(), m);
                            }
                        });
                    if self.selected_mode != old_mode {
                        self.rebuild_automaton();
                    }

                    // Pattern
                    ui.label("Pattern:");
                    let patterns: Vec<&str> = self.automaton.available_patterns().to_vec();
                    let old_pat = self.selected_pattern.clone();
                    egui::ComboBox::from_id_salt("pattern_combo")
                        .selected_text(&self.selected_pattern)
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &p in &patterns {
                                ui.selectable_value(&mut self.selected_pattern, p.to_owned(), p);
                            }
                        });
                    if self.selected_pattern != old_pat {
                        self.do_reset();
                        let pat = self.selected_pattern.clone();
                        self.automaton.load_pattern(&pat);
                    }

                    // Boundary
                    ui.label("Boundary:");
                    let old_bnd = self.selected_boundary;
                    egui::ComboBox::from_id_salt("boundary_combo")
                        .selected_text(self.selected_boundary.as_str())
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for &b in BoundaryMode::all() {
                                ui.selectable_value(&mut self.selected_boundary, b, b.as_str());
                            }
                        });
                    if self.selected_boundary != old_bnd {
                        self.automaton.set_boundary(self.selected_boundary);
                    }

                    // Custom rule editor
                    if self.selected_mode == "Custom Rules" {
                        ui.label("Rule (B/S):");
                        let old_rule = self.custom_rule_str.clone();
                        ui.text_edit_singleline(&mut self.custom_rule_str);
                        if self.custom_rule_str != old_rule {
                            let (b, s) = LifeLike::parse_bs(&self.custom_rule_str);
                            self.custom_birth = b;
                            self.custom_survival = s;
                            self.rebuild_automaton();
                        }
                    }

                    // Resize button
                    ui.add_space(4.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Resize Grid..."),
                        )
                        .clicked()
                    {
                        let (gw, gh) = {
                            let g = self.automaton.get_grid();
                            (g.width, g.height)
                        };
                        self.resize_w_input = gw.to_string();
                        self.resize_h_input = gh.to_string();
                        self.show_resize_dialog = true;
                    }

                    // RLE import from clipboard
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Import RLE"),
                        )
                        .on_hover_text("Paste RLE pattern from clipboard")
                        .clicked()
                    {
                        match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                            Ok(text) => {
                                let pts = parse_rle(&text);
                                if pts.is_empty() {
                                    self.set_status("Clipboard: no valid RLE found");
                                } else {
                                    let (gw, gh) = {
                                        let g = self.automaton.get_grid();
                                        (g.width as i32, g.height as i32)
                                    };
                                    let cx = gw / 2;
                                    let cy = gh / 2;
                                    self.push_undo();
                                    for (dx, dy) in pts {
                                        let x = cx + dx;
                                        let y = cy + dy;
                                        if x >= 0 && x < gw && y >= 0 && y < gh {
                                            self.automaton
                                                .get_grid_mut()
                                                .set(y as usize, x as usize, 1);
                                        }
                                    }
                                    self.set_status("RLE pattern imported");
                                }
                            }
                            Err(e) => self.set_status(format!("Clipboard error: {e}")),
                        }
                    }

                    // RLE export to clipboard
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Copy RLE"),
                        )
                        .on_hover_text("Copy current grid as RLE to clipboard")
                        .clicked()
                    {
                        let rle = self.grid_to_rle();
                        match arboard::Clipboard::new().and_then(|mut c| c.set_text(rle)) {
                            Ok(()) => self.set_status("RLE copied to clipboard"),
                            Err(e) => self.set_status(format!("Clipboard error: {e}")),
                        }
                    }
                });
        });

        ui.add_space(4.0);

        // ── View section ─────────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("View")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(format!("Speed: {} steps/s", self.speed));
                    ui.add(egui::Slider::new(&mut self.speed, 1..=200).show_value(false));

                    ui.label(format!("Cell size: {}px", self.cell_size as u32));
                    ui.horizontal(|ui| {
                        if ui.small_button("−").clicked() {
                            self.cell_size = (self.cell_size - 1.0).max(1.0);
                        }
                        ui.add(
                            egui::Slider::new(&mut self.cell_size, 1.0..=64.0).show_value(false),
                        );
                        if ui.small_button("+").clicked() {
                            self.cell_size = (self.cell_size + 1.0).min(64.0);
                        }
                    });

                    ui.checkbox(&mut self.show_grid, "Grid lines (G)");
                    ui.checkbox(&mut self.rounded_cells, "Rounded cells");
                    ui.checkbox(&mut self.show_aging, "Cell aging overlay");

                    if self.view_mode == ViewMode::Isometric {
                        ui.add_space(4.0);
                        ui.label(format!("Block height: {:.2}×", self.iso_height));
                        ui.add(
                            egui::Slider::new(&mut self.iso_height, 0.25..=4.0).show_value(false),
                        );
                    }

                    ui.add_space(4.0);
                    ui.label("Paint state:");
                    ui.add(
                        egui::Slider::new(&mut self.paint_state, 0..=8)
                            .clamping(egui::SliderClamping::Always),
                    );
                    ui.label(
                        egui::RichText::new("Left-click = paint   Right-click = erase")
                            .small()
                            .weak(),
                    );
                });
        });

        ui.add_space(4.0);

        // ── Statistics section ───────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Statistics")
                .default_open(true)
                .show(ui, |ui| {
                    if self.pop_history.len() > 1 {
                        let max_pop = self.pop_history.iter().copied().max().unwrap_or(1).max(1);
                        let min_pop = self.pop_history.iter().copied().min().unwrap_or(0);
                        let avg_pop =
                            self.pop_history.iter().sum::<usize>() / self.pop_history.len();

                        ui.label(format!(
                            "Peak: {}  Min: {}  Avg: {}",
                            max_pop, min_pop, avg_pop
                        ));

                        // Sparkline
                        let graph_size = Vec2::new(ui.available_width(), 56.0);
                        let (rect, _) = ui.allocate_exact_size(graph_size, Sense::hover());
                        let painter = ui.painter_at(rect);
                        let bg = if self.dark_mode {
                            Color32::from_rgb(20, 20, 30)
                        } else {
                            Color32::from_gray(240)
                        };
                        painter.rect_filled(rect, CornerRadius::same(3), bg);
                        let n = self.pop_history.len();
                        let step = rect.width() / n.max(2) as f32;
                        let accent = mode_accent_color(&self.selected_mode);
                        let pts: Vec<egui::Pos2> = self
                            .pop_history
                            .iter()
                            .enumerate()
                            .map(|(i, &p)| {
                                egui::pos2(
                                    rect.left() + i as f32 * step,
                                    rect.bottom()
                                        - (p as f32 / max_pop as f32) * (rect.height() - 4.0)
                                        - 2.0,
                                )
                            })
                            .collect();
                        painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5_f32, accent)));
                    } else {
                        ui.weak("Run the simulation to see statistics.");
                    }

                    if ui.small_button("Reset stats").clicked() {
                        self.pop_history.clear();
                    }
                });
        });

        ui.add_space(4.0);

        // ── Export section ───────────────────────────────────────────────────
        section_frame.show(ui, |ui| {
            egui::CollapsingHeader::new("Export & State")
                .default_open(false)
                .show(ui, |ui| {
                    if ui
                        .add_sized(
                            [ui.available_width(), button_h],
                            egui::Button::new("Export PNG..."),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PNG image", &["png"])
                            .save_file()
                        {
                            let cs = (self.cell_size as u32).max(1);
                            match export::export_png(self.automaton.get_grid(), cs, &path) {
                                Ok(()) => self.set_status(format!("Saved {}", path.display())),
                                Err(e) => self.set_status(format!("Export failed: {}", e)),
                            }
                        }
                    }

                    ui.separator();

                    // Save / load grid state
                    ui.horizontal(|ui| {
                        let w = (ui.available_width() - 8.0) / 2.0;
                        if ui
                            .add_sized([w, button_h], egui::Button::new("Save State..."))
                            .on_hover_text("Save current grid to a .lifegrid file")
                            .clicked()
                        {
                            self.save_state();
                        }
                        if ui
                            .add_sized([w, button_h], egui::Button::new("Load State..."))
                            .on_hover_text("Load a previously saved .lifegrid file")
                            .clicked()
                        {
                            self.load_state();
                        }
                    });

                    ui.separator();

                    ui.checkbox(&mut self.record_frames, "Record frames");
                    if self.record_frames {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} frames buffered (max 500)",
                                self.frame_buffer.len()
                            ))
                            .small(),
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.frame_buffer.is_empty(),
                            egui::Button::new("Export GIF..."),
                        )
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GIF animation", &["gif"])
                            .save_file()
                        {
                            let (gw, gh) = {
                                let g = self.automaton.get_grid();
                                (g.width, g.height)
                            };
                            let cs = (self.cell_size as u32).clamp(1, 8);
                            let delay = 1000 / self.speed.clamp(1, 30);
                            match export::export_gif(&self.frame_buffer, gw, gh, cs, delay, &path) {
                                Ok(()) => self.set_status(format!(
                                    "GIF saved ({} frames)",
                                    self.frame_buffer.len()
                                )),
                                Err(e) => self.set_status(format!("GIF failed: {}", e)),
                            }
                        }
                    }
                    if ui
                        .add_enabled(
                            !self.frame_buffer.is_empty(),
                            egui::Button::new("Clear frames"),
                        )
                        .clicked()
                    {
                        self.frame_buffer.clear();
                        self.set_status("Frame buffer cleared");
                    }
                });
        });
    }

    fn ui_canvas(&mut self, ui: &mut egui::Ui) {
        if self.view_mode == ViewMode::Isometric {
            self.ui_canvas_iso(ui);
            return;
        }

        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());

        // Record canvas rect for zoom-to-fit (uses last frame's value, fine).
        self.canvas_rect = rect;

        let cs = self.cell_size;
        let dark = self.dark_mode;
        let show_grid = self.show_grid;
        let show_aging = self.show_aging;
        let rounding = if self.rounded_cells {
            CornerRadius::same(((cs * 0.18).max(1.0)) as u8)
        } else {
            CornerRadius::ZERO
        };

        let (gw, gh, cells_snapshot, age_snapshot) = {
            let g = self.automaton.get_grid();
            (g.width, g.height, g.cells.clone(), self.age_grid.clone())
        };

        // ── Mouse-wheel zoom ─────────────────────────────────────────────────
        if response.hovered() {
            let scroll_y = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.1 {
                let old_cs = self.cell_size;
                let new_cs = (old_cs * (1.0 + scroll_y * 0.04)).clamp(1.0, 64.0);
                if let Some(cursor) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                    let rel = (cursor - rect.min) + self.viewport_offset;
                    self.viewport_offset = rel * (new_cs / old_cs) - (cursor - rect.min);
                }
                self.cell_size = new_cs;
            }
        }

        // ── Middle-click pan ─────────────────────────────────────────────────
        if response.dragged_by(egui::PointerButton::Middle) {
            self.viewport_offset -= response.drag_delta();
        }

        // Clamp viewport to grid bounds.
        let max_vx = (gw as f32 * self.cell_size - avail.x).max(0.0);
        let max_vy = (gh as f32 * self.cell_size - avail.y).max(0.0);
        self.viewport_offset.x = self.viewport_offset.x.clamp(0.0, max_vx);
        self.viewport_offset.y = self.viewport_offset.y.clamp(0.0, max_vy);

        let vox = self.viewport_offset.x;
        let voy = self.viewport_offset.y;
        let cs = self.cell_size; // re-read after zoom

        // ── Visible cell culling ─────────────────────────────────────────────
        let start_gx = ((vox / cs) as i64 - 1).max(0) as usize;
        let end_gx = (((vox + avail.x) / cs) as i64 + 2).min(gw as i64) as usize;
        let start_gy = ((voy / cs) as i64 - 1).max(0) as usize;
        let end_gy = (((voy + avail.y) / cs) as i64 + 2).min(gh as i64) as usize;

        let painter = ui.painter_at(rect);

        // ── Background ───────────────────────────────────────────────────────
        let bg = cell_color(0, dark);
        painter.rect_filled(rect, CornerRadius::ZERO, bg);

        // ── Cells ────────────────────────────────────────────────────────────
        for gy in start_gy..end_gy {
            for gx in start_gx..end_gx {
                let state = cells_snapshot[gy * gw + gx];
                if state == 0 {
                    continue;
                }
                let base = cell_color(state, dark);
                let color = if show_aging && !age_snapshot.is_empty() {
                    aging_color(base, age_snapshot[gy * gw + gx])
                } else {
                    base
                };
                let cell_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.left() - vox + gx as f32 * cs,
                        rect.top() - voy + gy as f32 * cs,
                    ),
                    Vec2::splat(cs),
                );
                painter.rect_filled(cell_rect, rounding, color);
            }
        }

        // ── Grid lines ───────────────────────────────────────────────────────
        if show_grid && cs >= 4.0 {
            let lc = if dark {
                Color32::from_gray(50)
            } else {
                Color32::from_gray(210)
            };
            let stroke = egui::Stroke::new(0.5_f32, lc);
            for gx in start_gx..=end_gx {
                let px = rect.left() - vox + gx as f32 * cs;
                if px >= rect.left() - 1.0 && px <= rect.right() + 1.0 {
                    painter.line_segment(
                        [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                        stroke,
                    );
                }
            }
            for gy in start_gy..=end_gy {
                let py = rect.top() - voy + gy as f32 * cs;
                if py >= rect.top() - 1.0 && py <= rect.bottom() + 1.0 {
                    painter.line_segment(
                        [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                        stroke,
                    );
                }
            }
        }

        // ── Hover highlight ──────────────────────────────────────────────────
        let accent = mode_accent_color(&self.selected_mode);
        if let Some(cursor) = response.hover_pos() {
            let hx = ((cursor.x - rect.left() + vox) / cs) as i64;
            let hy = ((cursor.y - rect.top() + voy) / cs) as i64;
            if hx >= 0 && hx < gw as i64 && hy >= 0 && hy < gh as i64 {
                let hover_rect = Rect::from_min_size(
                    egui::pos2(
                        rect.left() - vox + hx as f32 * cs,
                        rect.top() - voy + hy as f32 * cs,
                    ),
                    Vec2::splat(cs),
                );
                let state = cells_snapshot[hy as usize * gw + hx as usize];
                let hl = Color32::from_rgba_premultiplied(
                    accent.r() / 3,
                    accent.g() / 3,
                    accent.b() / 3,
                    60,
                );
                painter.rect_filled(hover_rect, rounding, hl);
                response
                    .clone()
                    .on_hover_text(format!("({}, {})  state: {}", hx, hy, state));
            }
        }

        // ── Selection overlay ────────────────────────────────────────────────
        if let Some((x0, y0, x1, y1)) = self.selection {
            let sx = rect.left() - vox + x0 as f32 * cs;
            let sy = rect.top() - voy + y0 as f32 * cs;
            let ex = rect.left() - vox + (x1 + 1) as f32 * cs;
            let ey = rect.top() - voy + (y1 + 1) as f32 * cs;
            let sel_rect = egui::Rect::from_min_max(egui::pos2(sx, sy), egui::pos2(ex, ey));
            let fill = Color32::from_rgba_premultiplied(100, 160, 255, 25);
            let stroke = egui::Stroke::new(1.5_f32, Color32::from_rgb(120, 180, 255));
            painter.rect_filled(sel_rect, CornerRadius::ZERO, fill);
            painter.rect_stroke(
                sel_rect,
                CornerRadius::ZERO,
                stroke,
                egui::StrokeKind::Outside,
            );
        }

        // ── Left-click / drag: paint ─────────────────────────────────────────
        if self.tool_mode == ToolMode::Paint {
            if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy_v = ((pos.y - rect.top() + voy) / cs) as i64;
                    if gx >= 0 && gx < gw as i64 && gy_v >= 0 && gy_v < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.automaton.get_grid_mut().set(
                            gy_v as usize,
                            gx as usize,
                            self.paint_state,
                        );
                    }
                }
            }

            // ── Right-click / drag: erase ─────────────────────────────────────
            if response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy_v = ((pos.y - rect.top() + voy) / cs) as i64;
                    if gx >= 0 && gx < gw as i64 && gy_v >= 0 && gy_v < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.automaton
                            .get_grid_mut()
                            .set(gy_v as usize, gx as usize, 0);
                    }
                }
            }

            // Reset drag-undo sentinel when not dragging.
            if !response.dragged() {
                self.drag_undo_pushed = false;
            }
        }

        // ── Select tool ──────────────────────────────────────────────────────
        if self.tool_mode == ToolMode::Select {
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let gx = ((pos.x - rect.left() + vox) / cs) as i64;
                    let gy = ((pos.y - rect.top() + voy) / cs) as i64;
                    if self.selection_anchor.is_none() {
                        self.selection_anchor = Some((gx, gy));
                    }
                    let (ax, ay) = self.selection_anchor.unwrap();
                    self.selection = Some((ax.min(gx), ay.min(gy), ax.max(gx), ay.max(gy)));
                }
            }
            if !response.dragged_by(egui::PointerButton::Primary) {
                self.selection_anchor = None;
            }
            // Single click without drag clears selection.
            if response.clicked() {
                self.selection = None;
            }
            // Right-click also clears selection.
            if response.secondary_clicked() {
                self.selection = None;
            }
        }
    }

    // ── Modal dialogs ────────────────────────────────────────────────────────

    // ── Isometric 3-D canvas ─────────────────────────────────────────────────

    fn ui_canvas_iso(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        self.canvas_rect = rect;

        // Mouse-wheel zoom (no cursor centering in iso view)
        if response.hovered() {
            let scroll_y = ui.ctx().input(|i| i.smooth_scroll_delta.y);
            if scroll_y.abs() > 0.1 {
                self.cell_size = (self.cell_size * (1.0 + scroll_y * 0.04)).clamp(1.0, 64.0);
            }
        }

        // Middle-click pan
        if response.dragged_by(egui::PointerButton::Middle) {
            self.iso_offset += response.drag_delta();
        }

        let cs = self.cell_size;
        let iso_h = self.iso_height;
        let dark = self.dark_mode;

        let (gw, gh, cells_snapshot, age_snapshot) = {
            let g = self.automaton.get_grid();
            (g.width, g.height, g.cells.clone(), self.age_grid.clone())
        };

        // Isometric projection: world (wx, wy, wz) → screen.
        // Moving +wx → screen right+down; +wy → screen left+down; +wz → screen up.
        let base_x =
            rect.center().x + self.iso_offset.x - (gw as f32 / 2.0 - gh as f32 / 2.0) * cs * 0.5;
        let base_y = rect.min.y + rect.height() * 0.38 + self.iso_offset.y
            - (gw as f32 / 2.0 + gh as f32 / 2.0) * cs * 0.25;

        let ipt = |wx: f32, wy: f32, wz: f32| -> egui::Pos2 {
            egui::pos2(
                base_x + (wx - wy) * cs * 0.5,
                base_y + (wx + wy) * cs * 0.25 - wz * cs * 0.5 * iso_h,
            )
        };

        // Helper: darken a colour by factor 0–1.
        let shade = |c: Color32, f: f32| -> Color32 {
            Color32::from_rgb(
                (c.r() as f32 * f) as u8,
                (c.g() as f32 * f) as u8,
                (c.b() as f32 * f) as u8,
            )
        };

        let painter = ui.painter_at(rect);

        // Background
        painter.rect_filled(rect, CornerRadius::ZERO, cell_color(0, dark));

        // Ground-grid lines
        if self.show_grid && cs >= 3.0 {
            let lc = if dark {
                Color32::from_gray(38)
            } else {
                Color32::from_gray(215)
            };
            let stroke = egui::Stroke::new(0.5_f32, lc);
            for gx in 0..=gw {
                let p1 = ipt(gx as f32, 0.0, 0.0);
                let p2 = ipt(gx as f32, gh as f32, 0.0);
                painter.line_segment([p1, p2], stroke);
            }
            for gy in 0..=gh {
                let p1 = ipt(0.0, gy as f32, 0.0);
                let p2 = ipt(gw as f32, gy as f32, 0.0);
                painter.line_segment([p1, p2], stroke);
            }
        }

        // Collect live cells and sort back→front: ascending (gx+gy), then ascending gx.
        let mut live: Vec<(usize, usize, u8)> = cells_snapshot
            .iter()
            .enumerate()
            .filter(|(_, &s)| s > 0)
            .map(|(i, &s)| (i % gw, i / gw, s))
            .collect();
        live.sort_unstable_by_key(|&(gx, gy, _)| (gx + gy, gx));

        for (gx, gy, state) in &live {
            let x = *gx as f32;
            let y = *gy as f32;
            let h = *state as f32;

            let base_c = if self.show_aging && age_snapshot.len() > gy * gw + gx {
                aging_color(cell_color(*state, dark), age_snapshot[gy * gw + gx])
            } else {
                cell_color(*state, dark)
            };

            // Right face (x+1 side — lower right of block)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x + 1.0, y, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, 0.0),
                    ipt(x + 1.0, y, 0.0),
                ],
                shade(base_c, 0.52),
                egui::Stroke::NONE,
            ));
            // Left face (y+1 side — lower left of block)
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x + 1.0, y + 1.0, 0.0),
                    ipt(x, y + 1.0, 0.0),
                ],
                shade(base_c, 0.72),
                egui::Stroke::NONE,
            ));
            // Top face
            painter.add(egui::Shape::convex_polygon(
                vec![
                    ipt(x, y, h),
                    ipt(x + 1.0, y, h),
                    ipt(x + 1.0, y + 1.0, h),
                    ipt(x, y + 1.0, h),
                ],
                base_c,
                egui::Stroke::NONE,
            ));
        }

        // Selection overlay (flat diamond outline on ground plane)
        if let Some((x0, y0, x1, y1)) = self.selection {
            let x0f = x0 as f32;
            let y0f = y0 as f32;
            let x1f = (x1 + 1) as f32;
            let y1f = (y1 + 1) as f32;
            let sel_pts = vec![
                ipt(x0f, y0f, 0.0),
                ipt(x1f, y0f, 0.0),
                ipt(x1f, y1f, 0.0),
                ipt(x0f, y1f, 0.0),
            ];
            painter.add(egui::Shape::closed_line(
                sel_pts,
                egui::Stroke::new(2.0_f32, Color32::from_rgb(120, 180, 255)),
            ));
        }

        // Hover highlight using inverse iso projection (ground plane, z=0)
        let screen_to_grid = |pos: egui::Pos2| -> (i64, i64) {
            let dx = (pos.x - base_x) / (cs * 0.5);
            let dy = (pos.y - base_y) / (cs * 0.25);
            (
                ((dx + dy) / 2.0).floor() as i64,
                ((dy - dx) / 2.0).floor() as i64,
            )
        };

        let accent = mode_accent_color(&self.selected_mode);
        if let Some(cursor) = response.hover_pos() {
            let (hx, hy) = screen_to_grid(cursor);
            if hx >= 0 && hx < gw as i64 && hy >= 0 && hy < gh as i64 {
                let state = cells_snapshot[hy as usize * gw + hx as usize];
                let hxf = hx as f32;
                let hyf = hy as f32;
                painter.add(egui::Shape::closed_line(
                    vec![
                        ipt(hxf, hyf, 0.0),
                        ipt(hxf + 1.0, hyf, 0.0),
                        ipt(hxf + 1.0, hyf + 1.0, 0.0),
                        ipt(hxf, hyf + 1.0, 0.0),
                    ],
                    egui::Stroke::new(1.5_f32, accent),
                ));
                response
                    .clone()
                    .on_hover_text(format!("({}, {})  state: {}", hx, hy, state));
            }
        }

        // Paint tool
        if self.tool_mode == ToolMode::Paint {
            if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
                    if gx >= 0 && gx < gw as i64 && gy >= 0 && gy < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.automaton.get_grid_mut().set(
                            gy as usize,
                            gx as usize,
                            self.paint_state,
                        );
                    }
                }
            }
            if response.dragged_by(egui::PointerButton::Secondary) || response.secondary_clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
                    if gx >= 0 && gx < gw as i64 && gy >= 0 && gy < gh as i64 {
                        if !self.drag_undo_pushed {
                            self.push_undo();
                            self.drag_undo_pushed = true;
                        }
                        self.automaton
                            .get_grid_mut()
                            .set(gy as usize, gx as usize, 0);
                    }
                }
            }
            if !response.dragged() {
                self.drag_undo_pushed = false;
            }
        }

        // Select tool
        if self.tool_mode == ToolMode::Select {
            if response.dragged_by(egui::PointerButton::Primary) {
                if let Some(pos) = response.interact_pointer_pos() {
                    let (gx, gy) = screen_to_grid(pos);
                    if self.selection_anchor.is_none() {
                        self.selection_anchor = Some((gx, gy));
                    }
                    let (ax, ay) = self.selection_anchor.unwrap();
                    self.selection = Some((ax.min(gx), ay.min(gy), ax.max(gx), ay.max(gy)));
                }
            }
            if !response.dragged_by(egui::PointerButton::Primary) {
                self.selection_anchor = None;
            }
            if response.clicked() {
                self.selection = None;
            }
        }
    }

    fn ui_run_n_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_run_n_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("Run N Steps")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Number of steps:");
                ui.text_edit_singleline(&mut self.run_n_input);
                ui.horizontal(|ui| {
                    if ui.button("Run").clicked() {
                        if let Ok(n) = self.run_n_input.trim().parse::<u64>() {
                            let n = n.min(100_000);
                            for _ in 0..n {
                                self.do_step();
                            }
                            self.set_status(format!("Ran {} steps", n));
                        }
                        self.show_run_n_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_run_n_dialog = false;
                    }
                });
            });
        if !open {
            self.show_run_n_dialog = false;
        }
    }

    fn ui_resize_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_resize_dialog {
            return;
        }
        let mut open = true;
        egui::Window::new("Resize Grid")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.text_edit_singleline(&mut self.resize_w_input);
                });
                ui.horizontal(|ui| {
                    ui.label("Height:");
                    ui.text_edit_singleline(&mut self.resize_h_input);
                });
                ui.horizontal(|ui| {
                    if ui.button("Resize").clicked() {
                        let w = self
                            .resize_w_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        let h = self
                            .resize_h_input
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(100)
                            .clamp(10, 1000);
                        let mut new_auto = Self::make_auto(
                            &self.selected_mode,
                            w,
                            h,
                            self.selected_boundary,
                            &self.custom_birth,
                            &self.custom_survival,
                        );
                        new_auto.load_pattern(&self.selected_pattern);
                        let n = new_auto.get_grid().cells.len();
                        self.automaton = new_auto;
                        self.generation = 0;
                        self.pop_history.clear();
                        self.running = false;
                        self.undo.clear();
                        self.age_grid = vec![0u32; n];
                        self.frame_buffer.clear();
                        self.viewport_offset = Vec2::ZERO;
                        self.set_status(format!("Grid resized to {}×{}", w, h));
                        self.show_resize_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_resize_dialog = false;
                    }
                });
            });
        if !open {
            self.show_resize_dialog = false;
        }
    }
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

        // Simulation tick
        if self.running {
            let step_dur = self.step_duration();
            let now = Instant::now();
            if now - self.last_step >= step_dur {
                self.do_step();
                self.last_step = now;
            }
            ctx.request_repaint_after(step_dur);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Modal dialogs (rendered above all panels)
        self.ui_run_n_dialog(&ctx);
        self.ui_resize_dialog(&ctx);
        self.ui_help_modal(&ctx);

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
        egui::Panel::left("controls")
            .resizable(true)
            .min_size(panel_min_width)
            .default_size(panel_default_width)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.ui_controls(ui);
                });
            });

        // Canvas
        egui::CentralPanel::default().show(ui, |ui| {
            self.ui_canvas(ui);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
}
