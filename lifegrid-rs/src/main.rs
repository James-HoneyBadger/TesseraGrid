mod app;
mod automata;
mod core;
mod export;
mod patterns;
mod rle;
mod sim;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LifeGrid")
            .with_inner_size([1200.0, 750.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "LifeGrid",
        options,
        Box::new(|cc| Ok(Box::new(app::LifeGridApp::new(cc)))),
    )
}
