fn main() {
    // 仅在目标平台为 Windows 时执行
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        // 设置图标路径（相对于项目根目录）
        res.set_icon("assets/icon.ico");
        
        // 添加 Windows 文件属性信息
        res.set("ProductName", "批量ping工具");
        res.set("FileDescription", "这是一个用 Rust 构建的高性能多目标ping工具，支持1000+ip同时ping");
        res.set("LegalCopyright", "Copyright © 2025, 黄允威");
        res.set("FileVersion", "1.0.0.0");
        
        // 如果需要交叉编译（例如在 Linux 上打 Windows 包），可通过环境变量指定 windres 路径
        if let Ok(windres_path) = std::env::var("WINDRES") {
            res.set_windres_path(&windres_path);
        }
        
        // 最后一次性编译并嵌入资源
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {}", e);
            // 可根据需要决定是否 panic
            // panic!("Failed to embed Windows resources");
        }
    }
}
