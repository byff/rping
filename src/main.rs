// Windows: hide console window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use gui::app::PingTestApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("PingTest | Powered by byff")
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| Ok(Box::new(PingTestApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    let image = eframe::icon_data::from_png_bytes(png_data);
    match image {
        Ok(icon) => icon,
        Err(_) => egui::IconData::default(),
    }
}
