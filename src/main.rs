mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use gui::app::RPingApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([700.0, 400.0])
            .with_title("RPing - 多目标Ping工具 | Powered by byff")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "RPing",
        options,
        Box::new(|cc| Ok(Box::new(RPingApp::new(cc)))),
    )
}
