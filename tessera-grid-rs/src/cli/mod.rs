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

mod args;
mod run;

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
        assert_eq!(cli.steps, 10);
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
