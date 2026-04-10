use rping_lib::{RPingApp, AppConfig};
use eframe::Renderer;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    // Check if debug mode is enabled
    let config = AppConfig::load();
    #[cfg(windows)]
    {
        if !config.debug_mode {
            // Try to hide console window on Windows
            // This is a best-effort approach - on Windows GUI apps,
            // the console is typically hidden when compiled with GUI subsystem
        }
    }

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
