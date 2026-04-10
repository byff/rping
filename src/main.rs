use rping_lib::{RPingApp, AppConfig};
use eframe::Renderer;

#[cfg(windows)]
fn hide_console_window() {
    use std::ptr;
    use winapi::um::wincon::{GetConsoleWindow, FreeConsole};
    use winapi::um::winuser::{ShowWindow, SW_HIDE};

    unsafe {
        let console = GetConsoleWindow();
        if !console.is_null() {
            ShowWindow(console, SW_HIDE);
        }
        // Also detach from console (don't show error dialogs)
        FreeConsole();
    }
}

#[cfg(not(windows))]
fn hide_console_window() {}

fn main() -> eframe::Result<()> {
    // On Windows, hide the console window unless in debug mode
    let config = AppConfig::load();
    if !config.debug_mode {
        hide_console_window();
    }

    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([900.0, 500.0])
            .with_title("RPing - 多目标Ping工具 | Powered by byff")
            .with_drag_and_drop(true),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "RPing",
        options,
        Box::new(|cc| Ok(Box::new(RPingApp::new(cc)))),
    )
}
