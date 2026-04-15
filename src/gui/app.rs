use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use slint::{Timer, TimerMode, Weak, ModelRc, ComponentHandle};
use parking_lot::RwLock;
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::AppConfig;
use crate::ping::{PingEngine, PingTarget, PingStats};
use crate::utils;
use crate::excel;

#[allow(unused_extern_crates)]
extern crate slint_generated_ui__app as slint_gen;

pub use slint_gen::TableRow;

pub struct PingTestApp {
    config: AppConfig,
    engine: Arc<RwLock<PingEngine>>,
    targets: Vec<PingTarget>,
    sorted_indices: Vec<usize>,
    address_input: String,
    is_running: bool,
    imported_file_path: Option<PathBuf>,
    imported_excel_data: Option<(Vec<String>, Vec<Vec<String>>)>,
    selected_ip_col: Option<usize>,
    status_msg: String,
    runtime: Option<tokio::runtime::Runtime>,
    last_import_dir: Option<PathBuf>,
    ip_warning_confirmed: bool,
    // Interior-mutated window reference - set by main.rs via set_window()
    window_rc: Rc<RefCell<Option<Weak<slint_gen::MainWindow>>>>,
    timer: Option<Timer>,
    sort_column: i32,
    sort_descending: bool,
}

impl PingTestApp {
    pub fn new(window_rc: Rc<RefCell<Option<Weak<slint_gen::MainWindow>>>>) -> Self {
        let config = AppConfig::load();
        let address_input = if config.remember_addresses {
            config.last_addresses.join("\n")
        } else {
            String::new()
        };

        let engine = PingEngine::new(
            config.ping.timeout_ms,
            config.ping.interval_ms,
            config.ping.packet_size,
            config.ping.max_concurrent,
        );

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            targets: Vec::new(),
            sorted_indices: Vec::new(),
            address_input,
            is_running: false,
            imported_file_path: None,
            imported_excel_data: None,
            selected_ip_col: None,
            status_msg: "就绪".to_string(),
            runtime: None,
            last_import_dir: None,
            ip_warning_confirmed: false,
            window_rc,
            timer: None,
            sort_column: 0,
            sort_descending: false,
        }
    }

    fn ensure_runtime(&mut self) -> &tokio::runtime::Runtime {
        if self.runtime.is_none() {
            self.runtime = Some(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime"),
            );
        }
        self.runtime.as_ref().unwrap()
    }

    pub fn start_ping(&mut self) {
        if self.address_input.trim().is_empty() {
            self.update_status("请输入IP地址");
            return;
        }

        let cleaned_input = utils::extract_and_clean_ips(&self.address_input);
        if cleaned_input.is_empty() {
            self.update_status("未找到有效IP地址");
            return;
        }

        let count = utils::count_cidr_ips(&cleaned_input);
        if count > 1000 && !self.ip_warning_confirmed {
            self.show_ip_warning();
            return;
        }

        let (parsed, _skipped) = utils::parse_targets(&cleaned_input, self.config.cidr_strip_first_last);
        if parsed.is_empty() {
            self.update_status("未解析到有效IP地址");
            return;
        }

        self.targets = parsed
            .into_iter()
            .enumerate()
            .map(|(i, (hostname, ip))| PingTarget {
                index: i,
                hostname,
                ip,
                stats: Arc::new(RwLock::new(PingStats::default())),
            })
            .collect();

        self.sorted_indices = (0..self.targets.len()).collect();
        self.ip_warning_confirmed = false;

        if self.config.remember_addresses {
            self.config.last_addresses = self.address_input
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.config.save();
        }

        let mut engine = PingEngine::new(
            self.config.ping.timeout_ms,
            self.config.ping.interval_ms,
            self.config.ping.packet_size,
            self.config.ping.max_concurrent,
        );
        engine.set_targets(self.targets.clone());

        self.ensure_runtime();
        engine.start(self.runtime.as_ref().unwrap().handle());
        *self.engine.write() = engine;

        self.is_running = true;
        self.update_status(format!("正在 Ping {} 个目标...", self.targets.len()));
        self.update_is_running(true);
        self.update_table_rows();
    }

    pub fn stop_ping(&mut self) {
        self.engine.read().stop();
        self.is_running = false;
        self.update_status("已停止");
        self.update_is_running(false);
    }

    pub fn refresh_ping(&mut self) {
        self.stop_ping();
        self.targets.clear();
        self.sorted_indices.clear();
        self.start_ping();
    }

    pub fn import_file(&mut self) {
        let initial_dir = self.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("所有支持格式", &["txt", "csv", "xlsx"])
            .add_filter("文本文件", &["txt", "csv"])
            .add_filter("Excel文件", &["xlsx"]);

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        match dialog.pick_file() {
            Some(path) => {
                self.handle_file_drop(path);
            }
            None => {
                #[cfg(target_os = "linux")]
                {
                    self.show_error("文件对话框无法打开", "请确保已安装 GTK3 和相关文件选择器组件：\n\nUbuntu/Debian: sudo apt install libgtk-3-0\nFedora: sudo dnf install gtk3\n或使用 snap/flatpak 安装");
                }
            }
        }
    }

    fn handle_file_drop(&mut self, path: PathBuf) {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(dir) = path.parent() {
            self.last_import_dir = Some(dir.to_path_buf());
        }

        match ext.as_str() {
            "txt" | "csv" => {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let cleaned = utils::extract_and_clean_ips(&content);
                        self.address_input = if cleaned.is_empty() { content } else { cleaned };
                        self.update_status(format!("已导入: {}", path.display()));
                        self.update_input_text(self.address_input.clone());
                        if self.is_running {
                            self.refresh_ping();
                        }
                    }
                    Err(e) => {
                        self.show_error("读取文件失败", &format!("文件: {}\n\n错误: {}", path.display(), e));
                    }
                }
            }
            "xlsx" => {
                match excel::read_excel(&path) {
                    Ok((headers, rows)) => {
                        let ip_cols = utils::find_ip_columns(&headers, &rows);
                        if ip_cols.is_empty() {
                            self.update_status("未在Excel中找到IP列");
                        } else if ip_cols.len() == 1 {
                            let col_idx = ip_cols[0].0;
                            self.extract_ips_from_excel(&rows, col_idx);
                            self.imported_file_path = Some(path);
                            self.imported_excel_data = Some((headers, rows));
                            self.selected_ip_col = Some(col_idx);
                            if self.is_running {
                                self.refresh_ping();
                            }
                        } else {
                            let col_idx = ip_cols[0].0;
                            self.extract_ips_from_excel(&rows, col_idx);
                            self.imported_file_path = Some(path);
                            self.imported_excel_data = Some((headers, rows));
                            self.selected_ip_col = Some(col_idx);
                        }
                    }
                    Err(e) => {
                        self.show_error("读取Excel失败", &format!("文件: {}\n\n错误: {}", path.display(), e));
                    }
                }
            }
            "xls" => {
                self.show_error("Excel 格式不支持", &format!("文件: {}\n\n.xls 格式暂不支持，请将文件另存为 .xlsx 后重试。", path.display()));
            }
            _ => {
                self.show_error("不支持的文件格式", &format!("文件: {}\n\n支持的格式: txt, csv, xlsx\n请选择正确的文件格式。", path.display()));
            }
        }
    }

    fn extract_ips_from_excel(&mut self, rows: &[Vec<String>], col_idx: usize) {
        let mut ips = Vec::new();
        for row in rows {
            if let Some(cell) = row.get(col_idx) {
                let trimmed = cell.trim();
                if !trimmed.is_empty() {
                    ips.push(trimmed.to_string());
                }
            }
        }
        self.address_input = ips.join("\n");
        self.update_input_text(self.address_input.clone());
        self.update_status(format!("已从Excel导入 {} 个地址", ips.len()));
    }

    pub fn export_results(&mut self) {
        let initial_dir = self.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Excel文件", &["xlsx"])
            .set_file_name("ping_results.xlsx");

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        match dialog.save_file() {
            Some(path) => {
                match excel::export_results(&path, &self.targets, &self.config.export) {
                    Ok(_) => self.update_status(format!("已导出: {}", path.display())),
                    Err(e) => self.show_error("导出失败", &format!("文件: {}\n\n错误: {}", path.display(), e)),
                }
            }
            None => {
                #[cfg(target_os = "linux")]
                {
                    self.show_error("文件对话框无法打开", "请确保已安装 GTK3 和相关文件选择器组件：\nUbuntu/Debian: sudo apt install libgtk-3-0\nFedora: sudo dnf install gtk3");
                }
            }
        }
    }

    pub fn export_to_source_excel(&mut self) {
        if let (Some(source_path), Some(_), Some(ip_col)) = (
            &self.imported_file_path.clone(),
            &self.imported_excel_data,
            self.selected_ip_col,
        ) {
            let initial_dir = self.last_import_dir.clone();
            let stem = source_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("result");
            let filename = format!("{}_ping_result.xlsx", stem);

            let mut dialog = rfd::FileDialog::new()
                .add_filter("Excel文件", &["xlsx"])
                .set_file_name(&filename);

            if let Some(dir) = initial_dir {
                dialog = dialog.set_directory(dir);
            }

            if let Some(path) = dialog.save_file() {
                match excel::insert_results_to_excel(
                    source_path,
                    &path,
                    &self.targets,
                    ip_col,
                    &self.config.export,
                ) {
                    Ok(_) => self.update_status(format!("已插入结果: {}", path.display())),
                    Err(e) => self.show_error("插入结果失败", &format!("文件: {}\n\n错误: {}", path.display(), e)),
                }
            } else {
                #[cfg(target_os = "linux")]
                {
                    self.show_error("文件对话框无法打开", "请确保已安装 GTK3：\n sudo apt install libgtk-3-0");
                }
            }
        }
    }

    fn update_status(&self, msg: &str) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_status_message(msg.into());
        }
    }

    fn update_is_running(&self, running: bool) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_is_running(running);
        }
    }

    fn update_input_text(&self, text: String) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_input_text(text.into());
        }
    }

    fn show_ip_warning(&self) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_ip_warning_dialog_open(true);
        }
    }

    fn show_error(&self, title: &str, msg: &str) {
        self.update_status(&format!("错误: {}", title));
        // Could show a dialog here, for now just update status
        let _ = (title, msg);
    }

    pub fn close_settings(&mut self) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_settings_dialog_open(false);
        }
    }

    // Called from save_settings callback - reads current values from window properties
    pub fn save_settings_from_window(&mut self) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            self.config.ping.timeout_ms = w.get_timeout_value() as u64;
            self.config.ping.packet_size = w.get_packet_size_value() as usize;
            self.config.ping.interval_ms = w.get_interval_value() as u64;
            self.config.ping.max_concurrent = w.get_max_concurrent_value() as usize;
            self.config.cidr_strip_first_last = w.get_cidr_enabled();
            self.config.remember_addresses = w.get_remember_addresses();
            self.config.debug_mode = w.get_debug_mode();
            self.config.save();
            w.set_settings_dialog_open(false);
        }
    }

    pub fn reset_settings(&mut self) {
        self.config = AppConfig::default();
        self.config.save();

        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            w.set_timeout_value(self.config.ping.timeout_ms as i32);
            w.set_packet_size_value(self.config.ping.packet_size as i32);
            w.set_interval_value(self.config.ping.interval_ms as i32);
            w.set_max_concurrent_value(self.config.ping.max_concurrent as i32);
            w.set_cidr_enabled(self.config.cidr_strip_first_last);
            w.set_remember_addresses(self.config.remember_addresses);
            w.set_debug_mode(self.config.debug_mode);
        }
    }

    pub fn sort_table(&mut self, column: i32) {
        if self.sort_column == column {
            self.sort_descending = !self.sort_descending;
        } else {
            self.sort_column = column;
            self.sort_descending = false;
        }

        let mut old_idx: Vec<usize> = self.sorted_indices.clone();
        match column {
            0 => {} // # - no sorting
            1 => {
                old_idx.sort_by(|&a, &b| {
                    let empty = String::new();
                    let ha = self.targets.get(a).map(|t| &t.hostname).unwrap_or(&empty);
                    let hb = self.targets.get(b).map(|t| &t.hostname).unwrap_or(&empty);
                    if self.sort_descending { hb.cmp(ha) } else { ha.cmp(hb) }
                });
            }
            2 => {
                old_idx.sort_by(|&a, &b| {
                    let ia = self.targets.get(a).map(|t| t.ip).unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0)));
                    let ib = self.targets.get(b).map(|t| t.ip).unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0)));
                    if self.sort_descending { ib.cmp(&ia) } else { ia.cmp(&ib) }
                });
            }
            3 => {
                old_idx.sort_by(|&a, &b| {
                    let sa = self.targets.get(a).map(|t| t.stats.read().success_count).unwrap_or(0);
                    let sb = self.targets.get(b).map(|t| t.stats.read().success_count).unwrap_or(0);
                    if self.sort_descending { sb.cmp(&sa) } else { sa.cmp(&sb) }
                });
            }
            4 => {
                old_idx.sort_by(|&a, &b| {
                    let fa = self.targets.get(a).map(|t| t.stats.read().fail_count).unwrap_or(0);
                    let fb = self.targets.get(b).map(|t| t.stats.read().fail_count).unwrap_or(0);
                    if self.sort_descending { fb.cmp(&fa) } else { fa.cmp(&fb) }
                });
            }
            _ => {}
        }
        self.sorted_indices = old_idx;
        self.update_table_rows();
    }

    fn update_table_rows(&self) {
        if let Some(w) = self.window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
            let rows = self.build_table_rows();
            w.set_table_rows(ModelRc::new(rows));
            w.set_selected_ip_count(self.targets.len() as i32);
        }
    }

    fn build_table_rows(&self) -> Vec<TableRow> {
        let mut rows = Vec::new();

        for &idx in &self.sorted_indices {
            if idx >= self.targets.len() {
                continue;
            }
            let target = &self.targets[idx];
            let stats = target.stats.read();

            let fail_pct = if stats.total_sent == 0 {
                "-".to_string()
            } else {
                format!("{:.0}%", (stats.fail_count as f64 / stats.total_sent as f64) * 100.0)
            };

            let rtt = stats.last_rtt_us.map(|u| format!("{:.1}ms", u as f64 / 1000.0)).unwrap_or_else(|| "-".to_string());
            let rtt_max = if stats.max_rtt_us > 0 { format!("{:.1}ms", stats.max_rtt_us as f64 / 1000.0) } else { "-".to_string() };
            let rtt_min = if stats.min_rtt_us > 0 && stats.success_count > 0 { format!("{:.1}ms", stats.min_rtt_us as f64 / 1000.0) } else { "-".to_string() };
            let rtt_avg = if stats.success_count > 0 { format!("{:.1}ms", stats.avg_rtt_us() as f64 / 1000.0) } else { "-".to_string() };

            let status = if stats.total_sent == 0 {
                -1
            } else if stats.fail_count == 0 {
                0
            } else if stats.success_count == 0 {
                2
            } else {
                1
            };

            rows.push(TableRow {
                num: (idx + 1) as i32,
                hostname: target.hostname.clone().into(),
                ip: target.ip.to_string().into(),
                success: stats.success_count as i32,
                fail: stats.fail_count as i32,
                fail_pct: fail_pct.into(),
                total: stats.total_sent as i32,
                rtt: rtt.into(),
                rtt_max: rtt_max.into(),
                rtt_min: rtt_min.into(),
                rtt_avg: rtt_avg.into(),
                status,
            });
        }

        rows
    }

    fn start_timer(&mut self) {
        let window_rc = self.window_rc.clone();
        let app_ptr = Arc::new(self as *mut PingTestApp);

        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
            let app = unsafe { &*app_ptr.as_ref() };
            if let Some(w) = window_rc.borrow().as_ref().and_then(|w| w.upgrade()) {
                let rows = app.build_table_rows();
                w.set_table_rows(ModelRc::new(rows));
                w.set_selected_ip_count(app.targets.len() as i32);
            }
        });
        self.timer = Some(timer);
    }

impl Default for PingTestApp {
    fn default() -> Self {
        Self::new(Rc::new(RefCell::new(None)))
    }
}
