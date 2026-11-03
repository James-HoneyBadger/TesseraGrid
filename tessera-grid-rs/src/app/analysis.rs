//! Isolated damage-spreading experiments and incremental elementary sweeps.

use std::time::{Duration, Instant};

use super::*;
use crate::core::analysis::{AnalysisMetadata, Metrics};
use crate::sim::{DamageExperiment, DamageSample};

const SENSITIVITY_MAX: u32 = 1000;
const SWEEP_WIDTH: usize = 128;
const SWEEP_HEIGHT: usize = 96;
const SWEEP_STEPS: u64 = 95;

pub(super) struct Sensitivity {
    pair: DamageExperiment,
    total: u32,
    samples: Vec<DamageSample>,
    finished: bool,
    cancelled: bool,
}

impl Sensitivity {
    fn new(source: &Simulation, total: u32) -> Result<Self, String> {
        let pair = DamageExperiment::new(source)?;
        let initial = pair.sample();
        Ok(Self {
            pair,
            total: total.clamp(1, SENSITIVITY_MAX),
            samples: vec![initial],
            finished: false,
            cancelled: false,
        })
    }

    fn advance(&mut self) {
        if self.finished {
            return;
        }
        let sample = self.pair.step();
        self.samples.push(sample);
        self.finished = sample.elapsed >= u64::from(self.total);
    }

    fn cancel(&mut self) {
        if !self.finished {
            self.finished = true;
            self.cancelled = true;
        }
    }
}

pub(super) struct SweepRow {
    rule: u8,
    metrics: Metrics,
}

impl SweepRow {
    fn observation(&self) -> &'static str {
        match self.metrics.cycle.map(|cycle| cycle.period) {
            Some(1) => "fixed point",
            Some(_) => "period detected",
            None => "not observed",
        }
    }
}

pub(super) struct SweepResult {
    rows: Vec<SweepRow>,
    boundary: BoundaryMode,
    current: Option<Simulation>,
    running: bool,
}

impl SweepResult {
    fn new(boundary: BoundaryMode) -> Self {
        Self {
            rows: Vec::new(),
            boundary,
            current: None,
            running: true,
        }
    }

    fn advance(&mut self) {
        if !self.running {
            return;
        }
        let rule = self.rows.len() as u8;
        let sim = self.current.get_or_insert_with(|| {
            let mut sim = Simulation::new(
                "Elementary 1D",
                SWEEP_WIDTH,
                SWEEP_HEIGHT,
                RuleSpec::new(self.boundary, &[3], &[2, 3]).with_elementary(rule),
                "Single Cell",
            );
            sim.record_timeline = false;
            sim.detect_cycles = true;
            sim
        });
        sim.step();
        if sim.generation >= SWEEP_STEPS {
            self.rows.push(SweepRow {
                rule,
                metrics: sim.metrics(),
            });
            self.current = None;
            self.running = self.rows.len() < 256;
        }
    }

    fn cancel(&mut self) {
        self.running = false;
        self.current = None;
    }

    fn to_csv(&self) -> String {
        let mut out = Vec::new();
        AnalysisMetadata::write_header(&mut out).expect("writing to memory");
        for row in &self.rows {
            let metadata = AnalysisMetadata {
                mode: "Elementary 1D".to_owned(),
                rule: format!("W{}", row.rule),
                boundary: self.boundary,
                width: SWEEP_WIDTH,
                height: SWEEP_HEIGHT,
                initialization: "Single Cell".to_owned(),
                seed: None,
            };
            metadata
                .write_row(&mut out, row.metrics)
                .expect("writing to memory");
        }
        String::from_utf8(out).expect("CSV is UTF-8")
    }
}

impl TesseraGridApp {
    pub(super) fn start_sensitivity(&mut self) {
        match Sensitivity::new(&self.sim, self.sensitivity_generations) {
            Ok(experiment) => {
                self.running = false;
                self.run_n_pending = 0;
                self.sensitivity = Some(experiment);
            }
            Err(err) => self.set_status(format!("Cannot start experiment: {err}")),
        }
    }

    pub(super) fn advance_sensitivity(&mut self, ctx: &egui::Context) {
        if !self.show_analysis {
            if let Some(experiment) = &mut self.sensitivity {
                experiment.cancel();
            }
            return;
        }
        let Some(experiment) = self
            .sensitivity
            .as_mut()
            .filter(|experiment| !experiment.finished)
        else {
            return;
        };
        let start = Instant::now();
        while !experiment.finished && start.elapsed() < Duration::from_millis(8) {
            experiment.advance();
        }
        if experiment.finished {
            let (y, x) = experiment.pair.target();
            self.set_status(format!(
                "Damage spreading measured from ({x}, {y}); working grid unchanged"
            ));
        } else {
            ctx.request_repaint();
        }
    }

    pub(super) fn advance_sweep(&mut self, ctx: &egui::Context) {
        if !self.show_analysis {
            if let Some(sweep) = &mut self.sweep {
                sweep.cancel();
            }
            return;
        }
        let Some(sweep) = self.sweep.as_mut().filter(|sweep| sweep.running) else {
            return;
        };
        let start = Instant::now();
        while sweep.running && start.elapsed() < Duration::from_millis(8) {
            sweep.advance();
        }
        if sweep.running {
            ctx.request_repaint();
        } else {
            self.set_status("Swept all 256 elementary rules");
        }
    }

    pub(super) fn ui_analysis(&mut self, ctx: &egui::Context) {
        if !self.show_analysis {
            return;
        }
        let mut open = true;
        egui::Window::new("Analysis")
            .open(&mut open)
            .collapsible(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                self.ui_sensitivity_section(ui);
                ui.separator();
                self.ui_sweep_section(ui);
            });
        self.show_analysis = open;
        if !open {
            if let Some(experiment) = &mut self.sensitivity {
                experiment.cancel();
            }
            if let Some(sweep) = &mut self.sweep {
                sweep.cancel();
            }
        }
    }

    fn ui_sensitivity_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Damage spreading from a one-cell change");
        ui.weak("Runs two independent copies of the current grid. Does not change the working grid or prove chaos.");
        ui.horizontal(|ui| {
            let busy = self
                .sensitivity
                .as_ref()
                .is_some_and(|experiment| !experiment.finished);
            ui.add_enabled_ui(!busy, |ui| {
                ui.label("Generations:");
                ui.add(
                    egui::Slider::new(&mut self.sensitivity_generations, 10..=SENSITIVITY_MAX)
                        .logarithmic(true),
                );
                if ui.button("Run experiment").clicked() {
                    self.start_sensitivity();
                    ui.ctx().request_repaint();
                }
            });
            if busy && ui.button("Stop").clicked() {
                if let Some(experiment) = &mut self.sensitivity {
                    experiment.cancel();
                }
            }
        });
        let Some(experiment) = &self.sensitivity else {
            ui.weak("No experiment has been run yet.");
            return;
        };
        let sample = experiment.pair.sample();
        let source = experiment.pair.control();
        ui.label(format!(
            "{} / {} / {} boundary",
            source.mode(),
            source.rule_string(),
            source.boundary().as_str()
        ));
        ui.add(
            egui::ProgressBar::new(sample.elapsed as f32 / experiment.total as f32)
                .desired_width(240.0)
                .show_percentage(),
        );
        ui.label(format!(
            "{} differing cells ({:.2}%) after {} steps",
            sample.distance,
            sample.fraction * 100.0,
            sample.elapsed
        ));
        if let Some(rate) = experiment.pair.log_separation_rate() {
            ui.label(format!(
                "Finite-time log separation: {rate:.4} per generation (not a chaos classification)"
            ));
        } else if sample.elapsed > 0 {
            ui.label("This perturbation died out; other initial states may behave differently.");
        }
        if experiment.cancelled {
            ui.weak("Cancelled; showing partial results.");
        }
        let points: Vec<[f64; 2]> = experiment
            .samples
            .iter()
            .map(|sample| [sample.elapsed as f64, sample.fraction])
            .collect();
        egui_plot::Plot::new("sensitivity_plot")
            .height(150.0)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .allow_boxed_zoom(false)
            .include_y(0.0)
            .include_y(1.0)
            .x_axis_label("steps from initial state")
            .y_axis_label("fraction of active cells differing")
            .show(ui, |plot_ui| {
                plot_ui.line(egui_plot::Line::new("damage", points).width(1.5));
            });
        if ui.button("Copy damage CSV").clicked() {
            let mut csv = Vec::new();
            DamageExperiment::write_csv_header(&mut csv).expect("writing to memory");
            for sample in &experiment.samples {
                experiment
                    .pair
                    .write_csv_sample(&mut csv, *sample)
                    .expect("writing to memory");
            }
            let csv = String::from_utf8(csv).expect("CSV is UTF-8");
            match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(csv)) {
                Ok(()) => self.set_status("Damage CSV copied to clipboard"),
                Err(err) => self.set_status(format!("Clipboard error: {err}")),
            }
        }
    }

    fn ui_sweep_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Elementary rule sweep");
        ui.weak(format!("All 256 rules at {SWEEP_WIDTH}x{SWEEP_HEIGHT} for {SWEEP_STEPS} generations. Observations are not Wolfram classes."));
        ui.horizontal(|ui| {
            let busy = self.sweep.as_ref().is_some_and(|sweep| sweep.running);
            if ui
                .add_enabled(!busy, egui::Button::new("Run sweep"))
                .clicked()
            {
                self.sweep = Some(SweepResult::new(self.selected_boundary));
                ui.ctx().request_repaint();
            }
            if busy && ui.button("Stop sweep").clicked() {
                if let Some(sweep) = &mut self.sweep {
                    sweep.cancel();
                }
            }
            let has_rows = self
                .sweep
                .as_ref()
                .is_some_and(|sweep| !sweep.rows.is_empty());
            if ui
                .add_enabled(has_rows, egui::Button::new("Copy CSV"))
                .clicked()
            {
                if let Some(result) = &self.sweep {
                    let csv = result.to_csv();
                    match arboard::Clipboard::new()
                        .and_then(|mut clipboard| clipboard.set_text(csv))
                    {
                        Ok(()) => self.set_status("Sweep CSV copied to clipboard"),
                        Err(err) => self.set_status(format!("Clipboard error: {err}")),
                    }
                }
            }
        });
        let Some(result) = &self.sweep else {
            return;
        };
        ui.label(format!(
            "{} / 256 rules; {} boundary{}",
            result.rows.len(),
            result.boundary.as_str(),
            if !result.running && result.rows.len() < 256 {
                " (cancelled)"
            } else {
                ""
            }
        ));
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("sweep_grid")
                    .num_columns(5)
                    .striped(true)
                    .spacing([16.0, 2.0])
                    .show(ui, |ui| {
                        for header in [
                            "rule",
                            "cycle observation",
                            "population",
                            "entropy",
                            "period",
                        ] {
                            ui.label(egui::RichText::new(header).strong().small());
                        }
                        ui.end_row();
                        for row in &result.rows {
                            ui.label(row.rule.to_string());
                            ui.label(row.observation());
                            ui.label(row.metrics.population.to_string());
                            ui.label(format!("{:.3}", row.metrics.entropy));
                            ui.label(
                                row.metrics.cycle.map_or_else(
                                    || "-".to_owned(),
                                    |cycle| cycle.period.to_string(),
                                ),
                            );
                            ui.end_row();
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Simulation {
        Simulation::new(
            "Elementary 1D",
            8,
            8,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]).with_elementary(204),
            "Single Cell",
        )
    }

    #[test]
    fn sensitivity_is_isolated_and_includes_generation_zero() {
        let mut source = source();
        source.run_steps(3, || false);
        let original = source.state_cells();
        let mut experiment = Sensitivity::new(&source, 4).unwrap();
        let mut reference = DamageExperiment::new(&source).unwrap();
        assert_eq!(experiment.samples[0].elapsed, 0);
        assert_eq!(experiment.samples[0].distance, 1);
        source.reset();
        for _ in 0..4 {
            experiment.advance();
            assert_eq!(experiment.pair.sample(), reference.step());
        }
        assert!(experiment.finished);
        assert_ne!(source.state_cells(), original);
        assert_eq!(experiment.pair.control().population(), 1);
        assert!((experiment.pair.log_separation_rate().unwrap() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn cancelling_retains_samples_and_stops_execution() {
        let mut experiment = Sensitivity::new(&source(), 10).unwrap();
        experiment.advance();
        experiment.cancel();
        let before = experiment.pair.sample();
        experiment.advance();
        assert_eq!(experiment.pair.sample(), before);
        assert!(experiment.cancelled);
    }

    #[test]
    fn sweep_is_incremental_and_cancellable() {
        let mut sweep = SweepResult::new(BoundaryMode::Fixed);
        sweep.advance();
        assert_eq!(sweep.current.as_ref().unwrap().generation, 1);
        assert!(sweep.rows.is_empty());
        for _ in 1..SWEEP_STEPS {
            sweep.advance();
        }
        assert_eq!(sweep.rows.len(), 1);
        assert_eq!(sweep.rows[0].observation(), "fixed point");
        let csv = sweep.to_csv();
        assert_eq!(csv.lines().count(), 2);
        assert!(csv.contains("\"W0\",Fixed,128,96,\"Single Cell\""));
        sweep.cancel();
        sweep.advance();
        assert_eq!(sweep.rows.len(), 1);
        assert!(sweep.current.is_none());
    }

    #[test]
    fn analysis_window_renders_and_closing_cancels_both_jobs() {
        let ctx = egui::Context::default();
        let mut app = TesseraGridApp::from_config(AppConfig::default(), None);
        app.sim = source();
        app.show_analysis = true;
        app.start_sensitivity();
        app.sweep = Some(SweepResult::new(BoundaryMode::Reflect));
        let (cells, aux, generation) = (
            app.sim.state_cells(),
            app.sim.aux_state(),
            app.sim.generation,
        );
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.ui_analysis(ui.ctx()));
        output.textures_delta.clear();
        app.advance_sensitivity(&ctx);
        assert_eq!(app.sim.state_cells(), cells);
        assert_eq!(app.sim.aux_state(), aux);
        assert_eq!(app.sim.generation, generation);
        app.start_sensitivity();
        app.show_analysis = false;
        app.advance_sensitivity(&ctx);
        app.advance_sweep(&ctx);
        assert!(app.sensitivity.as_ref().unwrap().cancelled);
        assert_eq!(app.sensitivity.as_ref().unwrap().samples.len(), 1);
        assert!(!app.sweep.as_ref().unwrap().running);
        assert!(app.sweep.as_ref().unwrap().current.is_none());
    }

    #[test]
    fn completed_sweep_covers_every_rule_in_order() {
        let mut sweep = SweepResult::new(BoundaryMode::Wrap);
        while sweep.running {
            sweep.advance();
        }
        assert_eq!(sweep.rows.len(), 256);
        for (index, row) in sweep.rows.iter().enumerate() {
            assert_eq!(usize::from(row.rule), index);
            assert_eq!(row.metrics.generation, SWEEP_STEPS);
        }
        assert_eq!(sweep.to_csv().lines().count(), 257);
        let before = sweep.to_csv();
        sweep.advance();
        assert_eq!(sweep.to_csv(), before);
    }
}
