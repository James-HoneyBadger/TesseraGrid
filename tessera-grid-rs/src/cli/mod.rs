//! Headless command-line mode.
//!
//! Running with `--headless` (or any output/step flag) drives [`Simulation`]
//! without opening a window, which also makes the engine easy to script and test.

use std::path::PathBuf;

use clap::Parser;

use crate::automata::{LifeLike, ALL_MODES};
use crate::core::analysis::hamming_distance;
use crate::core::BoundaryMode;
use crate::export;
use crate::rle;
use crate::sim::{RuleSpec, Simulation, MAX_GRID_SIDE, MIN_GRID_SIDE};

const DEFAULT_MODE: &str = "Conway's Game of Life";

mod args;
mod run;

/// `println!` that ignores a closed pipe.
///
/// `println!` panics when the reader has gone away, so `tessera --analyze |
/// head` used to abort with "failed printing to stdout" instead of stopping
/// quietly. Every line the CLI prints goes through here.
macro_rules! emitln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout().lock(), $($arg)*);
    }};
}
pub(crate) use emitln;

/// Write a block of pre-formatted text, ignoring a closed pipe.
pub(crate) fn write_stdout(text: &str) {
    use std::io::Write as _;
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// Parse a Wolfram code such as `W110` (or just `110`).
///
/// Returns `Ok(None)` when the text is not a Wolfram code, so callers can fall
/// back to B/S notation.
pub(crate) fn parse_wolfram(value: &str) -> Result<Option<u8>, String> {
    let trimmed = value.trim();
    let digits = trimmed.strip_prefix(['W', 'w']).unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    let rule: u16 = digits
        .parse()
        .map_err(|_| format!("'{value}' is not a valid Wolfram rule"))?;
    u8::try_from(rule)
        .map(Some)
        .map_err(|_| format!("Wolfram rule {rule} is out of range (0–255)"))
}

pub use args::Cli;
pub use run::run;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headless_flags() {
        let cli =
            Cli::try_parse_from(["tessera", "--headless", "--steps", "10", "--png", "out.png"])
                .expect("parse");
        assert!(cli.is_headless());
        assert_eq!(cli.steps(), 10);
        assert_eq!(cli.png.as_deref(), Some(std::path::Path::new("out.png")));
    }

    #[test]
    fn no_flags_means_gui() {
        let cli = Cli::try_parse_from(["tessera"]).expect("parse");
        assert!(!cli.is_headless());
    }

    #[test]
    fn configuration_flags_imply_headless() {
        // `--mode` alone used to open the GUI and ignore the flag.
        for args in [
            vec!["tessera", "--mode", "High Life"],
            vec!["tessera", "--pattern", "Glider Gun"],
            vec!["tessera", "--width", "40"],
            vec!["tessera", "--height", "40"],
            vec!["tessera", "--boundary", "fixed"],
        ] {
            let cli = Cli::try_parse_from(&args).expect("parse");
            assert!(cli.is_headless(), "{args:?} should run headless");
        }
    }

    #[test]
    fn output_formatting_flags_imply_headless() {
        // These used to be ignored while the GUI opened.
        for args in [
            vec!["tessera", "--cell-size", "8"],
            vec!["tessera", "--gif-fps", "20"],
            vec!["tessera", "--steps", "0"],
        ] {
            let cli = Cli::try_parse_from(&args).expect("parse");
            assert!(cli.is_headless(), "{args:?} should run headless");
        }
    }

    #[test]
    fn export_defaults_apply_when_flags_are_omitted() {
        let cli = Cli::try_parse_from(["tessera", "--stats"]).expect("parse");
        assert_eq!(cli.steps(), 0);
        assert_eq!(cli.cell_size(), 4);
        assert_eq!(cli.gif_fps(), 10);
    }

    #[test]
    fn defaults_apply_when_geometry_is_omitted() {
        let cli = Cli::try_parse_from(["tessera", "--stats"]).expect("parse");
        assert_eq!(cli.grid_width(), 100);
        assert_eq!(cli.grid_height(), 100);
        assert_eq!(cli.boundary_mode().expect("default"), BoundaryMode::Wrap);
    }

    #[test]
    fn an_unknown_boundary_is_rejected() {
        let cli = Cli::try_parse_from(["tessera", "--boundary", "pentagon"]).expect("parse");
        assert!(cli.boundary_mode().is_err());
    }

    #[test]
    fn rule_implies_custom_mode() {
        let cli = Cli::try_parse_from(["tessera", "--rule", "B36/S23", "--stats"]).expect("parse");
        assert!(cli.is_headless());
        let (birth, survival) =
            LifeLike::try_parse_bs(cli.rule.as_deref().unwrap()).expect("valid rule");
        assert_eq!(birth, vec![3, 6]);
        assert_eq!(survival, vec![2, 3]);
    }
}
