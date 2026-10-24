//! egui application shell: state, helpers and panel composition.
//!
//! Rendering lives in the child modules; all simulation work is delegated to
//! [`crate::sim::Simulation`] so the GUI and CLI share one engine.

mod actions;
mod canvas;
mod config;
mod dialogs;
mod input;
mod io;
mod menu_bar;
mod selection;
mod shell;
mod sidebar;
mod statusbar;
mod theme;
mod timeline;
mod toolbar;

use std::time::{Duration, Instant};

pub(crate) use crate::theme::aging_color;
use crate::theme::Theme;

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
// Application state
// ---------------------------------------------------------------------------

pub struct TesseraGridApp {
    // ── Simulation ──────────────────────────────────────────────────────────
    sim: Simulation,
    running: bool,
    last_step: Instant,

    // ── Settings ────────────────────────────────────────────────────────────
    speed: u32,
    cell_size: f32,
    show_grid: bool,
    theme: Theme,
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

    // ── Pattern editor ───────────────────────────────────────────────────────
    show_editor: bool,
    editor: patterns::PatternDraft,

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
    theme_key: Option<(Theme, bool, Color32)>,
}

impl TesseraGridApp {
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
            theme: Theme::from_name(&cfg.theme).unwrap_or(if cfg.dark_mode {
                Theme::Dark
            } else {
                Theme::Light
            }),
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
            show_editor: false,
            editor: patterns::PatternDraft::new(24),
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

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), Instant::now()));
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------
