fn main() {
    // Set Windows exe icon when targeting Windows
    // cfg(windows) only checks HOST, not target — use CARGO_CFG_TARGET_OS instead
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {}", e);
        }
    }
}
