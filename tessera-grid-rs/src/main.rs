//! Tessera Grid — a cellular-automaton simulator built with egui.
//!
//! Launching without arguments opens the GUI; step/export flags switch to the
//! headless CLI. All engine and UI logic lives in the `tessera_grid` library.

use clap::Parser;

use tessera_grid::app::TesseraGridApp;
use tessera_grid::cli::{self, Cli};

fn main() -> eframe::Result<()> {
    let args = Cli::parse();
    if args.is_headless() {
        match cli::run(&args) {
            Ok(()) => return Ok(()),
            Err(err) => {
                eprintln!("tessera: {err}");
                std::process::exit(1);
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Tessera Grid")
            .with_app_id("tessera-grid")
            .with_inner_size([1200.0, 750.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "tessera-grid",
        options,
        Box::new(|cc| Ok(Box::new(TesseraGridApp::new(cc)))),
    )
}
