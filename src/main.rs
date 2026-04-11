// Windows: hide console window in release builds
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
            .with_title("PingTest | 批量Ping测试工具")
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
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => icon,
        Err(_) => egui::IconData::default(),
    }
}
