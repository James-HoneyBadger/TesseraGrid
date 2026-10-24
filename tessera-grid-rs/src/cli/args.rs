//! Argument definitions for the `tessera` binary.

use super::*;

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
