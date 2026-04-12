// Windows: hide console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use gui::app::PingTestApp;

fn write_file(path: &std::path::Path, content: &str) {
    let _ = std::fs::write(path, content);
}

fn main() {
    // Write startup marker - confirms binary at least reached main()
    if let Some(exe) = std::env::current_exe().ok() {
        write_file(&exe.with_file_name("pingtest_started.txt"), "started\n");
    }

    // Set up panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!("PANIC: {}\n", panic_info);
        if let Some(exe) = std::env::current_exe().ok() {
            write_file(&exe.with_file_name("pingtest_panic.log"), &msg);
        }
    }));

    // Initialize logging
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info")
    ).format_timestamp_millis()
     .try_init();

    log::info!("PingTest starting...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("PingTest | 批量Ping测试工具")
            .with_drag_and_drop(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    log::info!("Calling eframe::run_native...");

    if let Err(e) = eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| {
            log::info!("Creating PingTestApp...");
            Ok(Box::new(PingTestApp::new(cc)))
        }),
    ) {
        log::error!("eframe error: {:?}", e);
        if let Some(exe) = std::env::current_exe().ok() {
            write_file(&exe.with_file_name("pingtest_error.log"), &format!("eframe error: {:?}\n", e));
        }
    }

    log::info!("PingTest exited");
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => icon,
        Err(_) => egui::IconData::default(),
    }
}
