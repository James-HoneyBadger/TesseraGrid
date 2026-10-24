//! Execution of a headless run.

use super::*;

pub fn run(cli: &Cli) -> Result<(), String> {
    if cli.list_modes {
        for mode in ALL_MODES {
            println!("{mode}");
        }
        return Ok(());
    }

    let mode = if cli.rule.is_some() {
        "Custom Rules".to_owned()
    } else {
        cli.mode.clone().unwrap_or_else(|| DEFAULT_MODE.to_owned())
    };
    if !ALL_MODES.contains(&mode.as_str()) {
        return Err(format!(
            "unknown mode '{mode}'. Use --list-modes to see the available modes"
        ));
    }

    let (birth, survival) = match &cli.rule {
        Some(rule) => LifeLike::parse_bs(rule),
        None => (vec![3], vec![2, 3]),
    };

    let boundary = match cli.boundary.to_ascii_lowercase().as_str() {
        "wrap" | "toroidal" => BoundaryMode::Wrap,
        "fixed" | "dead" => BoundaryMode::Fixed,
        "reflect" | "mirror" => BoundaryMode::Reflect,
        other => {
            return Err(format!(
                "unknown boundary '{other}'. Expected wrap, fixed or reflect"
            ))
        }
    };

    let width = cli.width.clamp(1, 4096);
    let height = cli.height.clamp(1, 4096);
    let rule_spec = RuleSpec::new(boundary, &birth, &survival);
    let mut sim = Simulation::new(&mode, width, height, rule_spec, "");

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
    if cli.steps > 0 {
        sim.run_steps(cli.steps, || false);
    }

    let cell_size = cli.cell_size.clamp(1, 64);
    if let Some(path) = &cli.png {
        export::export_png(sim.grid(), cell_size, path)
            .map_err(|err| format!("PNG export failed: {err}"))?;
    }
    if let Some(path) = &cli.gif {
        if sim.frame_count() == 0 {
            return Err("GIF export needs at least one recorded step (use --steps)".to_owned());
        }
        let delay_ms = 1000 / cli.gif_fps.clamp(1, 60);
        export::export_gif(
            sim.frames(),
            sim.width(),
            sim.height(),
            cell_size,
            delay_ms,
            path,
        )
        .map_err(|err| format!("GIF export failed: {err}"))?;
    }
    if let Some(path) = &cli.rle {
        let encoded = rle::encode(sim.grid(), &sim.rule_string());
        std::fs::write(path, encoded)
            .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    }

    println!("mode: {}", sim.mode());
    println!("rule: {}", sim.rule_string());
    println!("grid: {}x{}", sim.width(), sim.height());
    println!("pattern: {pattern}");
    println!("generation: {}", sim.generation);
    println!("population: {}", sim.population());
    println!("density: {:.2}%", sim.density() * 100.0);
    if recording {
        println!("frames: {}", sim.frame_count());
    }

    Ok(())
}
