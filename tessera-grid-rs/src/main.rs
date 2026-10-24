mod app;
mod automata;
mod cli;
mod core;
mod export;
mod patterns;
mod rle;
mod sim;
mod theme;

use clap::Parser;

fn main() -> eframe::Result<()> {
    let args = cli::Cli::parse();
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
        Box::new(|cc| Ok(Box::new(app::TesseraGridApp::new(cc)))),
    )
}
