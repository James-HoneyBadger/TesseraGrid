//! Execution of a headless run.

use super::*;

pub fn run(cli: &Cli) -> Result<(), String> {
    if cli.list_modes {
        for mode in ALL_MODES {
            emitln!("{mode}");
        }
        return Ok(());
    }
    if cli.sweep_elementary {
        return run_elementary_sweep(cli);
    }

    // A Wolfram code selects the Elementary 1D mode, B/S notation selects Custom.
    let elementary = match cli.rule.as_deref() {
        Some(text) => parse_wolfram(text)?,
        None => None,
    };

    let mode = if elementary.is_some() {
        "Elementary 1D".to_owned()
    } else if cli.rule.is_some() {
        "Custom Rules".to_owned()
    } else {
        cli.mode.clone().unwrap_or_else(|| DEFAULT_MODE.to_owned())
    };
    if !ALL_MODES.contains(&mode.as_str()) {
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
    let width = cli.grid_width().clamp(MIN_GRID_SIDE, MAX_GRID_SIDE);
    let height = cli.grid_height().clamp(MIN_GRID_SIDE, MAX_GRID_SIDE);
    let mut rule_spec = RuleSpec::new(boundary, &birth, &survival);
    if let Some(rule) = elementary {
        rule_spec = rule_spec.with_elementary(rule);
    }
    let mut sim = Simulation::new(&mode, width, height, rule_spec, "");
    // Timeline snapshots are a UI affordance; a scripted run does not need one
    // grid clone per generation.
    sim.record_timeline = false;

    let pattern = cli.pattern.clone().unwrap_or_else(|| {
        sim.available_patterns()
            .first()
            .copied()
            .unwrap_or("")
            .to_owned()
    });
    if !sim.available_patterns().contains(&pattern.as_str()) && pattern != "Empty" {
        return Err(format!(
            "unknown pattern '{pattern}' for {mode}. Available: {}",
            sim.available_patterns().join(", ")
        ));
    }
    sim.load_pattern(&pattern);

    if cli.randomize || cli.seed.is_some() {
        let seed = cli.seed.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(1)
        });
        sim.randomize(seed);
    }

    let recording = cli.gif.is_some();
    sim.record_frames = recording;
    // Repeated-state detection is what `--analyze` and the CSV report need.
    sim.detect_cycles = cli.analyze || cli.analysis_csv.is_some();
    let steps = cli.steps();
    let mut series = Vec::new();
    if steps > 0 {
        if cli.analysis_csv.is_some() {
            for _ in 0..steps {
                sim.step();
                series.push(sim.metrics());
            }
        } else {
            sim.run_steps(steps, || false);
        }
    }

    let cell_size = cli.cell_size().clamp(1, 64);
    if let Some(path) = &cli.png {
        export::export_png(&sim.display_grid(), cell_size, path)
            .map_err(|err| format!("PNG export failed: {err}"))?;
    }
    if let Some(path) = &cli.gif {
        if sim.frame_count() == 0 {
            return Err("GIF export needs at least one recorded step (use --steps)".to_owned());
        }
        let delay_ms = 1000 / cli.gif_fps().clamp(1, 60);
        let (grid_w, grid_h) = (sim.width(), sim.height());
        export::export_gif(sim.frames(), grid_w, grid_h, cell_size, delay_ms, path)
            .map_err(|err| format!("GIF export failed: {err}"))?;
    }
    if let Some(path) = &cli.rle {
        let encoded = rle::encode(sim.grid(), &sim.rule_string());
        std::fs::write(path, encoded)
            .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    }

    if let Some(path) = &cli.analysis_csv {
        write_analysis_csv(path, &series)?;
    }
    if cli.analyze {
        print_analysis(&sim, steps);
    }

    emitln!("mode: {}", sim.mode());
    emitln!("rule: {}", sim.rule_string());
    emitln!("grid: {}x{}", sim.width(), sim.height());
    emitln!("pattern: {pattern}");
    emitln!("generation: {}", sim.generation);
    emitln!("population: {}", sim.population());
    emitln!("density: {:.2}%", sim.density() * 100.0);
    if recording {
        emitln!("frames: {}", sim.frame_count());
    }

    if let Some(generations) = cli.divergence {
        run_divergence(
            &mode,
            width,
            height,
            rule_spec,
            &pattern,
            cli.seed.unwrap_or(1),
            generations,
        )?;
    }

    Ok(())
}

/// Run every elementary rule and report one row of metrics per rule.
///
/// This is the cheapest way to see Wolfram's four classes side by side: stable
/// rules settle into a short cycle, chaotic rules keep high entropy and no cycle.
fn run_elementary_sweep(cli: &Cli) -> Result<(), String> {
    use rayon::prelude::*;

    let boundary = cli.boundary_mode()?;
    let width = cli.grid_width().clamp(MIN_GRID_SIDE, MAX_GRID_SIDE);
    let height = cli.grid_height().clamp(MIN_GRID_SIDE, MAX_GRID_SIDE);
    let steps = cli.steps().max(1);

    let rows: Vec<String> = (0..=255u8)
        .into_par_iter()
        .map(|rule| {
            let mut sim = Simulation::new(
                "Elementary 1D",
                width,
                height,
                RuleSpec::new(boundary, &[3], &[2, 3]).with_elementary(rule),
                "Single Cell",
            );
            sim.record_timeline = false;
            sim.detect_cycles = true;
            sim.run_steps(steps, || false);

            let metrics = sim.metrics();
            let (bounds_width, bounds_height) = metrics
                .bbox
                .map_or((0, 0), |(x0, y0, x1, y1)| (x1 - x0 + 1, y1 - y0 + 1));
            format!(
                "{rule},{},{:.6},{:.6},{bounds_width},{bounds_height},{}\n",
                metrics.population,
                metrics.density,
                metrics.entropy,
                metrics.cycle.map_or(0, |cycle| cycle.period)
            )
        })
        .collect();

    let mut table =
        String::from("rule,population,density,entropy,bounds_width,bounds_height,cycle_period\n");
    for row in rows {
        table.push_str(&row);
    }

    match &cli.analysis_csv {
        Some(path) => {
            std::fs::write(path, &table)
                .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
            emitln!(
                "swept 256 elementary rules ({steps} generations) into {}",
                path.display()
            );
        }
        None => write_stdout(&table),
    }
    Ok(())
}

/// Write one row per generation: the numbers a scientist would plot.
fn write_analysis_csv(
    path: &std::path::Path,
    series: &[crate::core::analysis::Metrics],
) -> Result<(), String> {
    let mut out =
        String::from("generation,population,density,entropy,bounds_width,bounds_height\n");
    for metrics in series {
        let (width, height) = metrics
            .bbox
            .map_or((0, 0), |(x0, y0, x1, y1)| (x1 - x0 + 1, y1 - y0 + 1));
        out.push_str(&format!(
            "{},{},{:.6},{:.6},{width},{height}\n",
            metrics.generation, metrics.population, metrics.density, metrics.entropy
        ));
    }
    std::fs::write(path, out).map_err(|err| format!("cannot write {}: {err}", path.display()))
}

fn print_analysis(sim: &Simulation, steps: u64) {
    let metrics = sim.metrics();
    emitln!("entropy: {:.4} bits/cell", metrics.entropy);
    match metrics.bbox {
        Some((x0, y0, x1, y1)) => {
            emitln!("bounds: {}x{} at ({x0}, {y0})", x1 - x0 + 1, y1 - y0 + 1)
        }
        None => emitln!("bounds: empty"),
    }
    match metrics.cycle {
        Some(cycle) => emitln!(
            "cycle: period {} after {} generations",
            cycle.period,
            cycle.transient
        ),
        None => emitln!("cycle: none within {steps} generations"),
    }
    if let Some(cycle) = metrics.cycle {
        if cycle.period == 1 {
            emitln!("note: the state is a still life (fixed point)");
        }
    }
}

/// Two runs that differ by a single cell, measured generation by generation.
///
/// A positive growth rate means nearby initial conditions separate, which is
/// the defining behaviour of a chaotic rule; a rate near zero means the rule is
/// stable and predictable.
#[allow(clippy::too_many_arguments)]
fn run_divergence(
    mode: &str,
    width: usize,
    height: usize,
    rules: RuleSpec<'_>,
    pattern: &str,
    seed: u64,
    steps: u64,
) -> Result<(), String> {
    let mut control = Simulation::new(mode, width, height, rules, pattern);
    let mut perturbed = Simulation::new(mode, width, height, rules, pattern);
    control.randomize(seed);
    perturbed.randomize(seed);

    // Flip the first live cell so the perturbation cannot be a no-op: flipping
    // an already-doomed cell can leave the two runs identical after a step.
    let (first_row, last_row) = control.active_rows();
    let mut target = (first_row, control.width() / 2);
    'search: for y in first_row..last_row {
        for x in 0..control.width() {
            if perturbed.grid().get(y, x) != 0 {
                target = (y, x);
                break 'search;
            }
        }
    }
    let current = perturbed.grid().get(target.0, target.1);
    perturbed.paint(target.0, target.1, if current == 0 { 1 } else { 0 });

    emitln!("# divergence: one perturbed cell, seed {seed}");
    emitln!("# generation,hamming_distance");
    let mut distance = 0;
    for generation in 1..=steps.max(1) {
        control.step();
        perturbed.step();
        distance = hamming_distance(&control.grid().cells, &perturbed.grid().cells);
        if generation <= 10 || generation % 10 == 0 || generation == steps {
            emitln!("{generation},{distance}");
        }
    }
    if distance > 0 {
        let rate = (distance as f64).ln() / steps.max(1) as f64;
        emitln!("# growth rate: {rate:.4} per generation (ln distance / steps)");
    } else {
        emitln!("# growth rate: 0 (the perturbation died out)");
    }
    Ok(())
}
