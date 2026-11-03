//! Argument definitions for the `tessera` binary.

use super::*;

/// Parse a `--boundary` value.
fn parse_boundary(value: &str) -> Result<BoundaryMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "wrap" | "toroidal" => Ok(BoundaryMode::Wrap),
        "fixed" | "dead" => Ok(BoundaryMode::Fixed),
        "reflect" | "mirror" => Ok(BoundaryMode::Reflect),
        other => Err(format!(
            "unknown boundary '{other}'. Expected wrap, fixed or reflect"
        )),
    }
}

/// Tessera Grid – a cellular-automaton simulator (GUI by default, headless on demand).
#[derive(Parser, Debug)]
#[command(
    name = "tessera",
    version,
    long_about = None,
    after_help = "Run without flags to open the GUI.\n\nExamples:\n  tessera --headless --steps 100 --png out.png\n  tessera --rule B36/S23 --randomize --seed 42 --gif out.gif\n  tessera --list-modes"
)]
pub struct Cli {
    /// Run without opening a window.
    #[arg(long)]
    pub headless: bool,

    /// Automaton mode (see --list-modes).
    #[arg(long, value_name = "MODE")]
    pub mode: Option<String>,

    /// B/S rule (B36/S23) or Wolfram code (W110); selects the matching mode.
    #[arg(long, value_name = "RULE")]
    pub rule: Option<String>,

    /// Starting pattern name (defaults to the first pattern for the mode).
    #[arg(long, value_name = "NAME")]
    pub pattern: Option<String>,

    /// Grid width in cells (default 100).
    #[arg(long, value_name = "N")]
    pub width: Option<usize>,

    /// Grid height in cells (default 100).
    #[arg(long, value_name = "N")]
    pub height: Option<usize>,

    /// Boundary mode: wrap, fixed or reflect (default wrap).
    #[arg(long, value_name = "MODE")]
    pub boundary: Option<String>,

    /// Generations to simulate before exporting (default 0).
    #[arg(long, value_name = "N")]
    pub steps: Option<u64>,

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

    /// Frames per second for --gif (default 10).
    #[arg(long, value_name = "N")]
    pub gif_fps: Option<u32>,

    /// Pixels per cell for PNG/GIF export (default 4).
    #[arg(long, value_name = "N")]
    pub cell_size: Option<u32>,

    /// Print final statistics to stdout.
    #[arg(long)]
    pub stats: bool,

    /// Print entropy, bounds and cycle information after the run.
    #[arg(long)]
    pub analyze: bool,

    /// Write metrics and run metadata as CSV (including generation 0 for time series).
    #[arg(long, value_name = "PATH")]
    pub analysis_csv: Option<PathBuf>,

    /// Measure active-cell damage for N steps from two copies of the initial state.
    #[arg(long, value_name = "N")]
    pub divergence: Option<u64>,

    /// Run all 256 elementary rules and print one metrics row per rule.
    #[arg(long)]
    pub sweep_elementary: bool,

    /// Repeat the run with different seeds and report mean and standard
    /// deviation, so a single run is not mistaken for evidence.
    #[arg(long, value_name = "N")]
    pub replicates: Option<u32>,

    /// List the available automaton modes and exit.
    #[arg(long)]
    pub list_modes: bool,
}

impl Cli {
    /// Reject requests that would otherwise be silently altered or bypassed.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            (
                "--width",
                self.grid_width() as u64,
                MIN_GRID_SIDE as u64,
                MAX_GRID_SIDE as u64,
            ),
            (
                "--height",
                self.grid_height() as u64,
                MIN_GRID_SIDE as u64,
                MAX_GRID_SIDE as u64,
            ),
            ("--cell-size", u64::from(self.cell_size()), 1, 64),
            ("--gif-fps", u64::from(self.gif_fps()), 1, 60),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be {min}–{max}, got {value}"));
            }
        }
        self.boundary_mode()?;
        if self.divergence == Some(0) {
            return Err("--divergence needs a positive step count".to_owned());
        }
        if let Some(count) = self.replicates {
            if !(1..=10_000).contains(&count) {
                return Err("--replicates must be 1–10000".to_owned());
            }
            if self.steps() == 0 {
                return Err("--replicates needs positive --steps".to_owned());
            }
            if self.sweep_elementary
                || self.divergence.is_some()
                || self.pattern.is_some()
                || self.png.is_some()
                || self.gif.is_some()
                || self.rle.is_some()
                || self.analyze
                || self.stats
                || self.cell_size.is_some()
                || self.gif_fps.is_some()
            {
                return Err("--replicates accepts mode/rule, geometry, boundary, --steps, --seed, --randomize and --analysis-csv only".to_owned());
            }
            if self
                .seed
                .unwrap_or(1)
                .checked_add(u64::from(count) - 1)
                .is_none()
            {
                return Err("replicate seed range exceeds u64".to_owned());
            }
        }
        if self.sweep_elementary {
            if self.steps() == 0 {
                return Err("--sweep-elementary needs positive --steps".to_owned());
            }
            if self.replicates.is_some()
                || self.divergence.is_some()
                || self.mode.is_some()
                || self.rule.is_some()
                || self.pattern.is_some()
                || self.randomize
                || self.seed.is_some()
                || self.png.is_some()
                || self.gif.is_some()
                || self.rle.is_some()
                || self.analyze
                || self.stats
                || self.cell_size.is_some()
                || self.gif_fps.is_some()
            {
                return Err("--sweep-elementary accepts geometry, boundary, --steps and --analysis-csv only".to_owned());
            }
        }
        if self.list_modes
            && (self.sweep_elementary
                || self.replicates.is_some()
                || self.divergence.is_some()
                || self.mode.is_some()
                || self.rule.is_some()
                || self.pattern.is_some()
                || self.randomize
                || self.seed.is_some()
                || self.steps.is_some()
                || self.width.is_some()
                || self.height.is_some()
                || self.boundary.is_some()
                || self.png.is_some()
                || self.gif.is_some()
                || self.rle.is_some()
                || self.stats
                || self.analyze
                || self.analysis_csv.is_some()
                || self.cell_size.is_some()
                || self.gif_fps.is_some())
        {
            return Err("--list-modes cannot be combined with simulation options".to_owned());
        }
        if self.gif.is_some() && self.steps() == 0 {
            return Err("GIF export needs at least one recorded step (use --steps)".to_owned());
        }
        if self.gif_fps.is_some() && self.gif.is_none() {
            return Err("--gif-fps requires --gif".to_owned());
        }
        if self.cell_size.is_some() && self.png.is_none() && self.gif.is_none() {
            return Err("--cell-size requires --png or --gif".to_owned());
        }
        let paths = [&self.png, &self.gif, &self.rle, &self.analysis_csv];
        for (index, path) in paths.iter().enumerate() {
            if path.is_some() && paths[..index].contains(path) {
                return Err("output paths must be distinct".to_owned());
            }
        }
        Ok(())
    }

    /// True when the invocation should not open a window.
    ///
    /// Every configuration flag counts, not just the output ones: passing
    /// `--mode` alone used to open the GUI and silently ignore the flag.
    pub fn is_headless(&self) -> bool {
        self.headless
            || self.list_modes
            || self.steps.is_some()
            || self.randomize
            || self.seed.is_some()
            || self.png.is_some()
            || self.gif.is_some()
            || self.rle.is_some()
            || self.stats
            || self.mode.is_some()
            || self.rule.is_some()
            || self.pattern.is_some()
            || self.width.is_some()
            || self.height.is_some()
            || self.boundary.is_some()
            || self.cell_size.is_some()
            || self.gif_fps.is_some()
            || self.analyze
            || self.analysis_csv.is_some()
            || self.divergence.is_some()
            || self.sweep_elementary
            || self.replicates.is_some()
    }

    /// Generations to simulate, applying the documented default.
    pub fn steps(&self) -> u64 {
        self.steps.unwrap_or(0)
    }

    /// Pixels per cell for image export, applying the documented default.
    pub fn cell_size(&self) -> u32 {
        self.cell_size.unwrap_or(4)
    }

    /// GIF frame rate, applying the documented default.
    pub fn gif_fps(&self) -> u32 {
        self.gif_fps.unwrap_or(10)
    }

    /// Grid width, applying the documented default.
    pub fn grid_width(&self) -> usize {
        self.width.unwrap_or(100)
    }

    /// Grid height, applying the documented default.
    pub fn grid_height(&self) -> usize {
        self.height.unwrap_or(100)
    }

    /// Boundary mode, applying the documented default.
    pub fn boundary_mode(&self) -> Result<BoundaryMode, String> {
        parse_boundary(self.boundary.as_deref().unwrap_or("wrap"))
    }
}
