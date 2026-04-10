fn main() {
    #[cfg(windows)]
    {
        use std::io;
        let manifest = winres::WindowsResource::new();
        manifest.set_subsystem(winres::Subsystem::WindowsGUI);

        // Try to set icon if it exists
        let exe_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let icon_path = std::path::Path::new(&exe_dir).join("assets/icon.ico");
        if icon_path.exists() {
            manifest.set_icon(icon_path.to_str().unwrap_or("")).ok();
        }

        // Write the resource file
        if let Err(e) = manifest.write() {
            eprintln!("winres warning: {}", e);
        }
    }
}
