// Windows: hide console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ping;
mod gui;
mod excel;
mod utils;

use gui::app::PingTestApp;

fn write_log(path: &std::path::Path, content: &str) {
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("Failed to write log {}: {}", path.display(), e);
    }
}

fn main() {
    let exe = std::env::current_exe().ok();
    let log_dir = exe.as_ref().map(|p| p.parent().unwrap_or(p));

    // Panic hook - writes to file before anything else
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!("[{}] PANIC: {}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            panic_info);
        eprintln!("{}", msg);
        if let Some(ref dir) = log_dir {
            write_log(&dir.join("pingtest_panic.log"), &msg);
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

    match eframe::run_native(
        "PingTest",
        options,
        Box::new(|cc| {
            log::info!("Creating PingTestApp...");
            Ok(Box::new(PingTestApp::new(cc)))
        }),
    ) {
        Ok(_) => {
            log::info!("PingTest exited normally");
        }
        Err(e) => {
            log::error!("eframe error: {:?}", e);
            if let Some(ref dir) = log_dir {
                write_log(&dir.join("pingtest_error.log"), &format!("eframe error: {:?}\n", e));
            }
        }
    }
}

fn load_icon() -> egui::IconData {
    let png_data = include_bytes!("../assets/rping.png");
    match eframe::icon_data::from_png_bytes(png_data) {
        Ok(icon) => icon,
        Err(e) => {
            log::warn!("Failed to load icon: {:?}", e);
            egui::IconData::default()
        }
    }
}
