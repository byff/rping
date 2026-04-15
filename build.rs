fn main() {
    // Compile Slint .slint files
    slint_build::compile("ui/app.slint")
        .unwrap_or_else(|e| {
            eprintln!("slint_build::compile failed: {}", e);
            std::process::exit(1);
        });

    // Only run winres when targeting MinGW (Linux->Windows cross-compile)
    // CARGO_CFG_TARGET_ENV=gnu indicates MinGW on any host
    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "gnu" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "批量ping工具");
        res.set("FileDescription", "高性能多目标ping工具，支持1000+IP同时ping");
        res.set("LegalCopyright", "Copyright 2025 byff");

        if let Ok(windres_path) = std::env::var("WINDRES") {
            res.set_windres_path(&windres_path);
        }

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {}", e);
        }
    }
}
