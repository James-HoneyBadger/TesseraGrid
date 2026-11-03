//! Execution of headless runs and reproducible experiments.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::*;
use crate::core::analysis::{AnalysisMetadata, Metrics};

pub fn run(cli: &Cli) -> Result<(), String> {
    cli.validate()?;
    if cli.list_modes {
        for mode in ALL_MODES {
            emitln!("{mode}");
        }
        return Ok(());
    }
    if cli.sweep_elementary {
        return run_elementary_sweep(cli);
    }

    let elementary = cli
        .rule
        .as_deref()
        .map(parse_wolfram)
        .transpose()?
        .flatten();
    let implied_mode = cli.rule.as_ref().map(|_| {
        if elementary.is_some() {
            "Elementary 1D"
        } else {
            "Custom Rules"
        }
    });
    if let (Some(mode), Some(implied)) = (&cli.mode, implied_mode) {
        if mode != implied {
            return Err(format!(
                "--rule selects '{implied}', which conflicts with --mode '{mode}'"
            ));
        }
    }
    let mode = implied_mode.or(cli.mode.as_deref()).unwrap_or(DEFAULT_MODE);
    if !ALL_MODES.contains(&mode) {
        return Err(format!(
            "unknown mode '{mode}'. Use --list-modes to see the available modes"
        ));
    }
    let (birth, survival) = match (&cli.rule, elementary) {
        (Some(rule), None) => {
            LifeLike::try_parse_bs(rule).map_err(|err| format!("invalid --rule: {err}"))?
        }
        _ => (vec![3], vec![2, 3]),
    };
    let boundary = cli.boundary_mode()?;
    let rules = RuleSpec::new(boundary, &birth, &survival)
        .with_elementary(elementary.unwrap_or(crate::automata::elementary::DEFAULT_RULE));
    let mut sim = Simulation::new(mode, cli.grid_width(), cli.grid_height(), rules, "Empty");
    sim.record_timeline = false;
    if cli.boundary.is_some() && sim.boundary() != boundary {
        return Err(format!("{mode} only supports wrap boundaries"));
    }
    if let Some(count) = cli.replicates {
        if mode == "Langton's Ant" {
            return Err(
                "--replicates is not supported for Langton's Ant: its initializer ignores seeds"
                    .to_owned(),
            );
        }
        return run_replicates(cli, mode, rules, count);
    }

    let pattern = cli
        .pattern
        .as_deref()
        .unwrap_or_else(|| sim.available_patterns()[0]);
    if !sim.available_patterns().contains(&pattern) {
        return Err(format!(
            "unknown pattern '{pattern}' for {mode}. Available: {}",
            sim.available_patterns().join(", ")
        ));
    }
    let randomize = cli.randomize || cli.seed.is_some() || pattern == "Random Soup";
    let seed = if randomize {
        Some(match cli.seed {
            Some(seed) => seed,
            None => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|err| format!("cannot generate a random seed: {err}"))?
                .as_nanos() as u64,
        })
    } else if mode == "Elementary 1D" && pattern == "Random Row" {
        Some(0x5eed_1234)
    } else {
        None
    };
    if randomize {
        if mode == "Langton's Ant" {
            return Err(
                "Langton's Ant has no randomized initializer; omit --randomize and --seed"
                    .to_owned(),
            );
        }
        sim.randomize(seed.expect("random initialization has a seed"));
    } else {
        sim.load_pattern(pattern);
    }
    let initialization = if randomize { "Random Soup" } else { pattern };
    let metadata = sim.analysis_metadata(initialization, seed);
    let experiment = cli
        .divergence
        .map(|_| DamageExperiment::new(&sim))
        .transpose()?;

    sim.record_frames = cli.gif.is_some();
    sim.detect_cycles = cli.analyze || cli.analysis_csv.is_some();
    let mut csv = open_metrics_csv(cli.analysis_csv.as_deref())?;
    if let Some(writer) = &mut csv {
        write_metrics(writer, &metadata, sim.metrics())?;
    }
    if csv.is_some() {
        for _ in 0..cli.steps() {
            sim.step();
            if let Some(writer) = &mut csv {
                write_metrics(writer, &metadata, sim.metrics())?;
            }
        }
    } else {
        sim.run_steps(cli.steps(), || false);
    }
    finish_csv(csv)?;

    if let Some(path) = &cli.png {
        export::export_png(&sim.display_grid(), cli.cell_size(), path)
            .map_err(|err| format!("PNG export failed: {err}"))?;
    }
    if let Some(path) = &cli.gif {
        let (width, height) = (sim.width(), sim.height());
        export::export_gif(
            sim.frames(),
            width,
            height,
            cli.cell_size(),
            1000 / cli.gif_fps(),
            path,
        )
        .map_err(|err| format!("GIF export failed: {err}"))?;
    }
    if let Some(path) = &cli.rle {
        std::fs::write(path, rle::encode(sim.grid(), &sim.rule_string()))
            .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    }
    if cli.analyze {
        print_analysis(&sim, cli.steps())?;
    }

    emitln!("mode: {}", sim.mode());
    emitln!("rule: {}", sim.rule_string());
    emitln!("boundary: {}", sim.boundary().as_str());
    emitln!("grid: {}x{}", sim.width(), sim.height());
    emitln!("pattern: {initialization}");
    if let Some(seed) = seed {
        emitln!("seed: {seed}");
    }
    emitln!("generation: {}", sim.generation);
    emitln!("population: {}", sim.population());
    emitln!("density: {:.2}%", sim.density() * 100.0);
    if sim.record_frames {
        emitln!("frames: {}", sim.frame_count());
    }
    if let (Some(experiment), Some(steps)) = (experiment, cli.divergence) {
        run_divergence(experiment, steps)?;
    }
    Ok(())
}

fn open_metrics_csv(path: Option<&Path>) -> Result<Option<BufWriter<File>>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let file =
        File::create(path).map_err(|err| format!("cannot create {}: {err}", path.display()))?;
    let mut writer = BufWriter::new(file);
    AnalysisMetadata::write_header(&mut writer)
        .map_err(|err| format!("CSV header failed: {err}"))?;
    Ok(Some(writer))
}

fn write_metrics(
    writer: &mut impl Write,
    metadata: &AnalysisMetadata,
    metrics: Metrics,
) -> Result<(), String> {
    metadata
        .write_row(writer, metrics)
        .map_err(|err| format!("CSV write failed: {err}"))
}

fn finish_csv(writer: Option<BufWriter<File>>) -> Result<(), String> {
    if let Some(mut writer) = writer {
        writer
            .flush()
            .map_err(|err| format!("CSV flush failed: {err}"))?;
    }
    Ok(())
}

fn run_replicates(cli: &Cli, mode: &str, rules: RuleSpec<'_>, count: u32) -> Result<(), String> {
    let mut csv = open_metrics_csv(cli.analysis_csv.as_deref())?;
    let base_seed = cli.seed.unwrap_or(1);
    let mut samples = Vec::with_capacity(count as usize);
    // One simulation at a time bounds grid and exact-cycle retention memory.
    for index in 0..count {
        let seed = base_seed + u64::from(index);
        let mut sim = Simulation::new(mode, cli.grid_width(), cli.grid_height(), rules, "Empty");
        sim.record_timeline = false;
        sim.detect_cycles = true;
        sim.randomize(seed);
        sim.run_steps(cli.steps(), || false);
        let metrics = sim.metrics();
        if let Some(writer) = &mut csv {
            write_metrics(
                writer,
                &sim.analysis_metadata("Random Soup", Some(seed)),
                metrics,
            )?;
        }
        samples.push(metrics);
    }
    finish_csv(csv)?;
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    let deviation = |values: &[f64]| {
        let average = mean(values);
        (values
            .iter()
            .map(|value| (value - average).powi(2))
            .sum::<f64>()
            / values.len() as f64)
            .sqrt()
    };
    let populations: Vec<f64> = samples
        .iter()
        .map(|metrics| metrics.population as f64)
        .collect();
    let entropies: Vec<f64> = samples.iter().map(|metrics| metrics.entropy).collect();
    let mut periods = std::collections::BTreeMap::<u64, usize>::new();
    for cycle in samples.iter().filter_map(|metrics| metrics.cycle) {
        *periods.entry(cycle.period).or_default() += 1;
    }
    emitln!(
        "# replicates: {count} (seeds {base_seed}..{}), random soup per seed, {} generations",
        base_seed + (u64::from(count) - 1),
        cli.steps()
    );
    emitln!("mode: {mode}");
    emitln!(
        "rule: {}",
        Simulation::new(mode, 1, 1, rules, "Empty").rule_string()
    );
    emitln!("boundary: {}", rules.boundary.as_str());
    emitln!("grid: {}x{}", cli.grid_width(), cli.grid_height());
    emitln!(
        "population: mean {:.1}  sd {:.1}  min {:.0}  max {:.0}",
        mean(&populations),
        deviation(&populations),
        populations.iter().copied().fold(f64::INFINITY, f64::min),
        populations
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    );
    emitln!(
        "entropy:    mean {:.4}  sd {:.4}",
        mean(&entropies),
        deviation(&entropies)
    );
    emitln!("# sd uses the population denominator N; metrics cover active cells");
    emitln!(
        "cycles:     {} of {count} observed (bounded exact-state retention)",
        periods.values().sum::<usize>()
    );
    if !periods.is_empty() {
        let histogram: Vec<_> = periods
            .iter()
            .map(|(period, count)| format!("period {period}: {count}"))
            .collect();
        emitln!("periods:    {}", histogram.join(", "));
    }
    Ok(())
}

fn run_elementary_sweep(cli: &Cli) -> Result<(), String> {
    let mut csv = open_metrics_csv(cli.analysis_csv.as_deref())?;
    if csv.is_none() {
        let mut header = Vec::new();
        AnalysisMetadata::write_header(&mut header)
            .map_err(|err| format!("CSV header failed: {err}"))?;
        write_stdout(&String::from_utf8(header).expect("CSV is UTF-8"))?;
    }
    for rule in 0..=255u8 {
        let mut sim = Simulation::new(
            "Elementary 1D",
            cli.grid_width(),
            cli.grid_height(),
            RuleSpec::new(cli.boundary_mode()?, &[3], &[2, 3]).with_elementary(rule),
            "Single Cell",
        );
        sim.record_timeline = false;
        sim.detect_cycles = true;
        sim.run_steps(cli.steps(), || false);
        let metadata = sim.analysis_metadata("Single Cell", None);
        if let Some(writer) = &mut csv {
            write_metrics(writer, &metadata, sim.metrics())?;
        } else {
            let mut row = Vec::new();
            write_metrics(&mut row, &metadata, sim.metrics())?;
            write_stdout(&String::from_utf8(row).expect("CSV is UTF-8"))?;
        }
    }
    finish_csv(csv)?;
    if let Some(path) = &cli.analysis_csv {
        emitln!(
            "swept 256 elementary rules ({} generations) into {}",
            cli.steps(),
            path.display()
        );
    }
    Ok(())
}

fn print_analysis(sim: &Simulation, steps: u64) -> Result<(), String> {
    let metrics = sim.metrics();
    emitln!("metric scope: active cells (Elementary: current row, y=0)");
    emitln!(
        "entropy: {:.4} bits/cell (state-frequency entropy, not spatial complexity)",
        metrics.entropy
    );
    match metrics.bbox {
        Some((x0, y0, x1, y1)) => {
            emitln!("bounds: {}x{} at ({x0}, {y0})", x1 - x0 + 1, y1 - y0 + 1)
        }
        None => emitln!("bounds: empty"),
    }
    match metrics.cycle {
        Some(cycle) => emitln!(
            "cycle: period {} (first retained occurrence at generation {})",
            cycle.period,
            cycle.transient
        ),
        None => emitln!(
            "cycle: none observed within {steps} generations (bounded exact-state retention)"
        ),
    }
    if metrics.cycle.is_some_and(|cycle| cycle.period == 1) {
        emitln!("note: the active evolution state is a fixed point");
    }
    Ok(())
}

fn run_divergence(mut experiment: DamageExperiment, steps: u64) -> Result<(), String> {
    let (y, x) = experiment.target();
    emitln!(
        "# divergence: damage spreading from configured initial state, perturbation at ({x}, {y})"
    );
    let mut buffer = Vec::new();
    DamageExperiment::write_csv_header(&mut buffer)
        .map_err(|err| format!("damage CSV header failed: {err}"))?;
    write_stdout(&String::from_utf8(buffer).expect("CSV is UTF-8"))?;
    let initial = experiment.sample();
    write_damage_sample(&experiment, initial)?;
    for _ in 0..steps {
        let sample = experiment.step();
        write_damage_sample(&experiment, sample)?;
    }
    match experiment.log_separation_rate() {
        Some(rate) => emitln!("# log separation rate: {rate:.6} per generation (ln distance / steps; not proof of chaos)"),
        None => emitln!("# log separation rate: undefined (this perturbation died out)"),
    }
    Ok(())
}

fn write_damage_sample(
    experiment: &DamageExperiment,
    sample: crate::sim::DamageSample,
) -> Result<(), String> {
    let mut buffer = Vec::new();
    experiment
        .write_csv_sample(&mut buffer, sample)
        .map_err(|err| format!("damage CSV write failed: {err}"))?;
    write_stdout(&String::from_utf8(buffer).expect("CSV is UTF-8"))
}
