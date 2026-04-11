fn main() {
    // Set Windows exe icon when targeting Windows
    // cfg(windows) only checks HOST, not target — use CARGO_CFG_TARGET_OS instead
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");

        // Tell winres where to find windres (cross-compilation on Linux)
        if let Ok(windres) = std::env::var("WINDRES") {
            res.set_windres_path(&windres);
        }

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {}", e);
        }
    }
}
