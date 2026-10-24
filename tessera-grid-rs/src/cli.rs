//! Headless command-line mode.
//!
//! Running with `--headless` (or any output/step flag) drives [`Simulation`]
//! without opening a window, which also makes the engine easy to script and test.

use std::path::PathBuf;

use clap::Parser;

use crate::automata::{LifeLike, ALL_MODES};
use crate::core::BoundaryMode;
use crate::export;
use crate::rle;
use crate::sim::{RuleSpec, Simulation};

const DEFAULT_MODE: &str = "Conway's Game of Life";

/// Tessera Grid – a cellular-automaton simulator (GUI by default, headless on demand).
#[derive(Parser, Debug)]
#[command(name = "tessera", version, about, long_about = None)]
pub struct Cli {
    /// Run without opening a window.
    #[arg(long)]
    pub headless: bool,

    /// Automaton mode (see --list-modes).
    #[arg(long, value_name = "MODE")]
    pub mode: Option<String>,

    /// Custom B/S rule, e.g. B36/S23. Implies --mode "Custom Rules".
    #[arg(long, value_name = "B/S")]
    pub rule: Option<String>,

    /// Starting pattern name (defaults to the first pattern for the mode).
    #[arg(long, value_name = "NAME")]
    pub pattern: Option<String>,

    /// Grid width in cells.
    #[arg(long, default_value_t = 100)]
    pub width: usize,

    /// Grid height in cells.
    #[arg(long, default_value_t = 100)]
    pub height: usize,

    /// Boundary mode: wrap, fixed or reflect.
    #[arg(long, default_value = "wrap")]
    pub boundary: String,

    /// Generations to simulate before exporting.
    #[arg(long, default_value_t = 0)]
    pub steps: u64,

    /// Fill the grid with a random soup before stepping.
    #[arg(long)]
    pub randomize: bool,

    /// Seed for --randomize (implies --randomize).
    #[arg(long)]
    pub seed: Option<u64>,

    /// Write a PNG snapshot to this path.
    #[arg(long, value_name = "PATH")]
    pub png: Option<PathBuf>,

    /// Record and write an animated GIF to this path.
    #[arg(long, value_name = "PATH")]
    pub gif: Option<PathBuf>,

    /// Write the final grid as RLE to this path.
    #[arg(long, value_name = "PATH")]
    pub rle: Option<PathBuf>,

    /// Frames per second for --gif.
    #[arg(long, default_value_t = 10)]
    pub gif_fps: u32,

    /// Pixels per cell for PNG/GIF export.
    #[arg(long, default_value_t = 4)]
    pub cell_size: u32,

    /// Print final statistics to stdout.
    #[arg(long)]
    pub stats: bool,

    /// List the available automaton modes and exit.
    #[arg(long)]
    pub list_modes: bool,
}

impl Cli {
    /// True when the invocation should not open a window.
    pub fn is_headless(&self) -> bool {
        self.headless
            || self.list_modes
            || self.steps > 0
            || self.randomize
            || self.seed.is_some()
            || self.png.is_some()
            || self.gif.is_some()
            || self.rle.is_some()
            || self.stats
    }
}

/// Execute a headless run. Returns a process exit error message on failure.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headless_flags() {
        let cli =
            Cli::try_parse_from(["tessera", "--headless", "--steps", "10", "--png", "out.png"])
                .expect("parse");
        assert!(cli.is_headless());
        assert_eq!(cli.steps, 10);
        assert_eq!(cli.png.as_deref(), Some(std::path::Path::new("out.png")));
    }

    #[test]
    fn no_flags_means_gui() {
        let cli = Cli::try_parse_from(["tessera"]).expect("parse");
        assert!(!cli.is_headless());
    }

    #[test]
    fn rule_implies_custom_mode() {
        let cli = Cli::try_parse_from(["tessera", "--rule", "B36/S23", "--stats"]).expect("parse");
        assert!(cli.is_headless());
        let (birth, survival) = LifeLike::parse_bs(cli.rule.as_deref().unwrap());
        assert_eq!(birth, vec![3, 6]);
        assert_eq!(survival, vec![2, 3]);
    }
}
