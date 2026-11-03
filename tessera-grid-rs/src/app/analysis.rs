//! Analysis panel: sensitivity experiments and the elementary rule sweep.
//!
//! Both tools exist on the command line; this is the interactive version, so an
//! educator can point at what is on screen instead of retyping parameters.

use std::time::{Duration, Instant};

use super::*;
use crate::core::analysis::hamming_distance;

const SENSITIVITY_MAX: u32 = 1000;
/// The sweep uses a fixed window so results are fast and comparable.
const SWEEP_WIDTH: usize = 128;
const SWEEP_HEIGHT: usize = 96;
const SWEEP_STEPS: u64 = 95;

/// Two runs that differ by one cell, advanced together with the main grid.
pub(super) struct Sensitivity {
    probe: Simulation,
    target: (usize, usize),
    total: u32,
    done: u32,
    distances: Vec<u64>,
    pub(super) finished: bool,
}

impl Sensitivity {
    /// Growth rate of the difference, in nats per generation.
    ///
    /// The experiment starts from a single differing cell, so `ln(distance)`
    /// divided by the number of generations is the separation rate.
    fn growth_rate(&self) -> Option<f64> {
        let last = *self.distances.last()?;
        (last > 0 && self.done > 1).then(|| (last as f64).ln() / f64::from(self.done - 1))
    }
}

/// One row of the elementary sweep table.
pub(super) struct SweepRow {
    pub rule: u8,
    pub population: u64,
    pub entropy: f64,
    pub period: u64,
    pub bounds: (usize, usize),
}

impl SweepRow {
    /// Rough Wolfram class, from the measurements.
    ///
    /// This is a heuristic for browsing, not a classifier: class 4 rules (rule
    /// 110) can look chaotic for a long time before showing structure.
    fn class(&self) -> &'static str {
        match self.period {
            1 => "fixed",
            2.. => "periodic",
            0 if self.entropy >= 0.5 => "chaotic",
            _ => "complex",
        }
    }
}

pub(super) struct SweepResult {
    pub rows: Vec<SweepRow>,
}

impl SweepResult {
    fn to_csv(&self) -> String {
        let mut out =
            String::from("rule,class,population,entropy,bounds_width,bounds_height,cycle_period\n");
        for row in &self.rows {
            out.push_str(&format!(
                "{},{},{},{:.6},{},{},{}\n",
                row.rule,
                row.class(),
                row.population,
                row.entropy,
                row.bounds.0,
                row.bounds.1,
                row.period
            ));
        }
        out
    }
}

impl TesseraGridApp {
    /// Start an experiment from the grid that is on screen.
    pub(super) fn start_sensitivity(&mut self) {
        let cells = self.sim.state_cells();
        let aux = self.sim.aux_state();
        let generation = self.sim.generation;
        let rules = RuleSpec::new(
            self.selected_boundary,
            &self.custom_birth,
            &self.custom_survival,
        )
        .with_elementary(self.elementary_rule);

        let mut probe = Simulation::new(
            &self.selected_mode,
            self.sim.width(),
            self.sim.height(),
            rules,
            "",
        );
        probe.record_timeline = false;
        probe.restore_state(&cells, &aux, generation);

        // Flip the first live cell, so the perturbation cannot be a no-op.
        let (first_row, last_row) = probe.active_rows();
        let width = probe.width();
        let mut target = (first_row, width / 2);
        'search: for y in first_row..last_row {
            for x in 0..width {
                if probe.grid().get(y, x) != 0 {
                    target = (y, x);
                    break 'search;
                }
            }
        }
        let current = probe.grid().get(target.0, target.1);
        probe.paint(target.0, target.1, if current == 0 { 1 } else { 0 });

        self.running = false;
        self.run_n_pending = 0;
        self.sensitivity = Some(Sensitivity {
            probe,
            target,
            total: self.sensitivity_generations.clamp(1, SENSITIVITY_MAX),
            done: 0,
            distances: Vec::new(),
            finished: false,
        });
    }

    /// Advance the experiment in short slices so the UI keeps repainting.
    pub(super) fn advance_sensitivity(&mut self, ctx: &egui::Context) {
        let Some(mut experiment) = self.sensitivity.take() else {
            return;
        };
        if experiment.finished {
            self.sensitivity = Some(experiment);
            return;
        }

        let slice_start = Instant::now();
        while experiment.done < experiment.total {
            self.sim.step();
            experiment.probe.step();
            experiment
                .distances
                .push(
                    hamming_distance(&self.sim.grid().cells, &experiment.probe.grid().cells) as u64,
                );
            experiment.done += 1;
            if slice_start.elapsed() >= Duration::from_millis(8) {
                break;
            }
        }
        experiment.finished = experiment.done >= experiment.total;
        if experiment.finished {
            let (y, x) = experiment.target;
            self.set_status(format!("Sensitivity measured from the cell at ({x}, {y})"));
        }
        let finished = experiment.finished;
        self.sensitivity = Some(experiment);
        if finished {
            self.metrics_cache = None;
        } else {
            ctx.request_repaint();
        }
    }

    /// Run all 256 elementary rules on a fixed window.
    pub(super) fn run_elementary_sweep(&mut self) {
        use rayon::prelude::*;

        let boundary = self.selected_boundary;
        let rows: Vec<SweepRow> = (0..=255u8)
            .into_par_iter()
            .map(|rule| {
                let mut sim = Simulation::new(
                    "Elementary 1D",
                    SWEEP_WIDTH,
                    SWEEP_HEIGHT,
                    RuleSpec::new(boundary, &[3], &[2, 3]).with_elementary(rule),
                    "Single Cell",
                );
                sim.record_timeline = false;
                sim.detect_cycles = true;
                sim.run_steps(SWEEP_STEPS, || false);

                let metrics = sim.metrics();
                SweepRow {
                    rule,
                    population: metrics.population,
                    entropy: metrics.entropy,
                    period: metrics.cycle.map_or(0, |cycle| cycle.period),
                    bounds: metrics
                        .bbox
                        .map_or((0, 0), |(x0, y0, x1, y1)| (x1 - x0 + 1, y1 - y0 + 1)),
                }
            })
            .collect();

        self.sweep = Some(SweepResult { rows });
        self.set_status("Swept all 256 elementary rules");
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
    }

    fn ui_sensitivity_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sensitivity to a one-cell change");
        ui.label(
            egui::RichText::new(
                "Copies the grid, flips one live cell and runs both forward. \
                 A rising separation means the rule is chaotic.",
            )
            .small()
            .weak(),
        );

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
                }
            });
            if busy && ui.button("Stop").clicked() {
                self.sensitivity = None;
            }
        });

        let Some(experiment) = &self.sensitivity else {
            ui.weak("No experiment has been run yet.");
            return;
        };

        let progress = experiment.done as f32 / experiment.total.max(1) as f32;
        ui.add(
            egui::ProgressBar::new(progress)
                .desired_width(240.0)
                .show_percentage(),
        );
        if let Some(rate) = experiment.growth_rate() {
            ui.label(format!(
                "Separation grew to {} cells · {rate:.4} nats/generation",
                experiment.distances.last().copied().unwrap_or(0)
            ));
        } else if experiment.finished {
            ui.label("The difference died out: this rule is stable under perturbation.");
        }

        if experiment.distances.len() > 1 {
            let points: Vec<[f64; 2]> = experiment
                .distances
                .iter()
                .enumerate()
                .map(|(index, &distance)| [index as f64 + 1.0, distance as f64])
                .collect();
            egui_plot::Plot::new("sensitivity_plot")
                .height(150.0)
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false)
                .include_y(0.0)
                .x_axis_label("generation")
                .y_axis_label("differing cells")
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        egui_plot::Line::new("distance", points)
                            .color(self.theme.accent(&self.selected_mode))
                            .width(1.5),
                    );
                });
        }
    }

    fn ui_sweep_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Elementary rule sweep");
        ui.label(
            egui::RichText::new(format!(
                "All 256 rules at {SWEEP_WIDTH}×{SWEEP_HEIGHT} for {SWEEP_STEPS} generations."
            ))
            .small()
            .weak(),
        );
        ui.horizontal(|ui| {
            if ui.button("Run sweep").clicked() {
                self.run_elementary_sweep();
            }
            let has_result = self.sweep.is_some();
            if ui
                .add_enabled(has_result, egui::Button::new("Copy CSV"))
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
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("sweep_grid")
                    .num_columns(5)
                    .striped(true)
                    .spacing([16.0, 2.0])
                    .show(ui, |ui| {
                        for header in ["rule", "class", "population", "entropy", "period"] {
                            ui.label(egui::RichText::new(header).strong().small());
                        }
                        ui.end_row();
                        for row in &result.rows {
                            ui.label(row.rule.to_string());
                            ui.label(row.class());
                            ui.label(row.population.to_string());
                            ui.label(format!("{:.3}", row.entropy));
                            ui.label(if row.period == 0 {
                                "—".to_owned()
                            } else {
                                row.period.to_string()
                            });
                            ui.end_row();
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::BoundaryMode;

    fn row(period: u64, entropy: f64) -> SweepRow {
        SweepRow {
            rule: 110,
            population: 1,
            entropy,
            period,
            bounds: (1, 1),
        }
    }

    #[test]
    fn the_class_hint_follows_the_measurements() {
        assert_eq!(row(1, 0.9).class(), "fixed");
        assert_eq!(row(2, 0.5).class(), "periodic");
        assert_eq!(row(0, 0.9).class(), "chaotic");
        assert_eq!(row(0, 0.1).class(), "complex");
    }

    #[test]
    fn the_sweep_csv_has_a_header_and_one_row_per_rule() {
        let result = SweepResult {
            rows: vec![row(1, 0.5), row(0, 0.7)],
        };
        let csv = result.to_csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "header plus two rows");
        assert!(
            lines[0].starts_with("rule,class,population"),
            "{}",
            lines[0]
        );
        assert!(lines[1].contains(",fixed,"), "{}", lines[1]);
        assert!(lines[2].contains(",chaotic,"), "{}", lines[2]);
    }

    fn empty_experiment() -> Sensitivity {
        let probe = Simulation::new(
            "Conway's Game of Life",
            8,
            8,
            RuleSpec::new(BoundaryMode::Wrap, &[3], &[2, 3]),
            "Empty",
        );
        Sensitivity {
            probe,
            target: (0, 0),
            total: 10,
            done: 0,
            distances: Vec::new(),
            finished: false,
        }
    }

    #[test]
    fn growth_rate_is_the_log_of_the_final_distance() {
        let mut experiment = empty_experiment();
        assert!(experiment.growth_rate().is_none(), "no samples yet");

        experiment.distances = vec![1, 2, 4, 8];
        experiment.done = 4;
        let rate = experiment.growth_rate().expect("rate");
        assert!((rate - 8f64.ln() / 3.0).abs() < 1e-9, "rate = {rate}");

        experiment.distances = vec![1, 0];
        experiment.done = 2;
        assert!(
            experiment.growth_rate().is_none(),
            "a difference that died out has no growth rate"
        );
    }
}
