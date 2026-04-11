use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use eframe::egui;
use egui::{RichText, Layout, Align};
use egui::text_selection::CursorRange;
use egui::epaint::text::cursor::Cursor;

use crate::config::AppConfig;
use crate::ping::{PingEngine, PingTarget, PingStats};
use crate::gui::{table, dialogs, theme};
use crate::utils;
use crate::excel;

pub struct PingTestApp {
    config: AppConfig,
    engine: Arc<RwLock<PingEngine>>,
    targets: Vec<PingTarget>,
    sorted_indices: Vec<usize>,
    table_state: table::TableState,
    dialog_state: dialogs::DialogState,

    // Input state
    address_input: String,
    is_running: bool,

    // File import state
    imported_file_path: Option<PathBuf>,
    imported_excel_data: Option<(Vec<String>, Vec<Vec<String>>)>,
    selected_ip_col: Option<usize>,

    // Status
    status_msg: String,

    // Runtime - lazy init
    runtime: Option<tokio::runtime::Runtime>,

    // Theme applied
    theme_applied: bool,

    // Track if input was cleaned (avoid re-cleaning on every frame)
    last_cleaned_input: String,

    // Focus input on first frame
    input_focus_requested: bool,

    // Countdown to apply select-all after focus is granted
    select_all_countdown: u8,

    // Timestamp (secs) of last click on input TextEdit, for double-click detection
    last_input_click_secs: Option<f64>,
}

impl PingTestApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
            table_state: table::TableState::default(),
            dialog_state: dialogs::DialogState::default(),
            address_input: address_input.clone(),
            is_running: false,
            imported_file_path: None,
            imported_excel_data: None,
            selected_ip_col: None,
            status_msg: "就绪".to_string(),
            runtime: None,
            theme_applied: false,
            last_cleaned_input: address_input,
            input_focus_requested: false,
            select_all_countdown: 2,
            last_input_click_secs: None,
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

    /// Auto-clean is removed — parse_targets naturally skips invalid lines.
    /// Manual cleanup via 🧹 button or file import only.

    fn start_ping(&mut self) {
        if self.address_input.trim().is_empty() {
            self.status_msg = "请输入IP地址".to_string();
            return;
        }

        let count = utils::count_cidr_ips(&self.address_input);
        if count > 1000 && !self.dialog_state.ip_warning_confirmed {
            self.dialog_state.ip_count_warning = count;
            self.dialog_state.show_ip_warning = true;
            return;
        }

        let (parsed, skipped) = utils::parse_targets(&self.address_input, self.config.cidr_strip_first_last);
        if parsed.is_empty() {
            if skipped > 0 {
                self.dialog_state.show_error("解析失败", &format!(
                    "未解析到有效IP地址，跳过了 {} 行无效内容。\n\n请检查输入格式，每行一个IP/域名/CIDR。",
                    skipped
                ));
            } else {
                self.status_msg = "未解析到有效IP地址".to_string();
            }
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
        self.dialog_state.ip_warning_confirmed = false;

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
        if skipped > 0 {
            self.status_msg = format!("正在 Ping {} 个目标（跳过 {} 行无效内容）", self.targets.len(), skipped);
        } else {
            self.status_msg = format!("正在 Ping {} 个目标...", self.targets.len());
        }
    }

    fn stop_ping(&mut self) {
        self.engine.read().stop();
        self.is_running = false;
        self.status_msg = "已停止".to_string();
    }

    /// Refresh: stop current ping, re-parse input, restart
    fn refresh_ping(&mut self) {
        self.stop_ping();
        // Reset stats
        self.targets.clear();
        self.sorted_indices.clear();
        self.start_ping();
    }

    fn handle_file_drop(&mut self, path: PathBuf) {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if let Some(dir) = path.parent() {
            self.config.last_import_dir = Some(dir.to_path_buf());
            self.config.save();
        }

        match ext.as_str() {
            "txt" | "csv" => {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let cleaned = utils::extract_and_clean_ips(&content);
                        self.address_input = if cleaned.is_empty() { content } else { cleaned };
                        self.last_cleaned_input = self.address_input.clone();
                        self.status_msg = format!("已导入: {}", path.display());
                        if self.is_running {
                            self.refresh_ping();
                        }
                    }
                    Err(e) => {
                        self.dialog_state.show_error("读取文件失败", &format!(
                            "文件: {}\n\n错误: {}", path.display(), e
                        ));
                    }
                }
            }
            "xlsx" => {
                match excel::read_excel(&path) {
                    Ok((headers, rows)) => {
                        let ip_cols = utils::find_ip_columns(&headers, &rows);
                        if ip_cols.is_empty() {
                            self.status_msg = "未在Excel中找到IP列".to_string();
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
                            self.dialog_state.excel_columns = ip_cols;
                            self.dialog_state.show_excel_column_picker = true;
                            self.imported_file_path = Some(path);
                            self.imported_excel_data = Some((headers, rows));
                        }
                    }
                    Err(e) => {
                        self.dialog_state.show_error("读取Excel失败", &format!(
                            "文件: {}\n\n错误: {}", path.display(), e
                        ));
                    }
                }
            }
            "xls" => {
                self.dialog_state.show_error("Excel 格式不支持", &format!(
                    "文件: {}\n\n.xls 格式暂不支持，请将文件另存为 .xlsx 后重试。",
                    path.display()
                ));
            }
            _ => {
                self.dialog_state.show_error("不支持的文件格式", &format!(
                    "文件: {}\n\n支持的格式: txt, csv, xlsx\n请选择正确的文件格式。",
                    path.display()
                ));
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
        self.last_cleaned_input = self.address_input.clone();
        self.status_msg = format!("已从Excel导入 {} 个地址", ips.len());
    }

    fn import_file(&mut self) {
        let initial_dir = self.config.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("所有支持格式", &["txt", "csv", "xlsx"])
            .add_filter("文本文件", &["txt", "csv"])
            .add_filter("Excel文件", &["xlsx"]);

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.pick_file() {
            self.handle_file_drop(path);
        }
    }

    fn export_results(&mut self) {
        let initial_dir = self.config.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Excel文件", &["xlsx"])
            .set_file_name("ping_results.xlsx");

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
            match excel::export_results(&path, &self.targets, &self.config.export) {
                Ok(_) => self.status_msg = format!("已导出: {}", path.display()),
                Err(e) => self.dialog_state.show_error("导出失败", &format!(
                    "文件: {}\n\n错误: {}", path.display(), e
                )),
            }
        }
    }

    fn export_to_source_excel(&mut self) {
        if let (Some(source_path), Some(_), Some(ip_col)) = (
            &self.imported_file_path.clone(),
            &self.imported_excel_data,
            self.selected_ip_col,
        ) {
            let initial_dir = self.config.last_import_dir.clone();
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
                    Ok(_) => self.status_msg = format!("已插入结果: {}", path.display()),
                    Err(e) => self.dialog_state.show_error("插入结果失败", &format!(
                        "文件: {}\n\n错误: {}", path.display(), e
                    )),
                }
            }
        }
    }
}

impl eframe::App for PingTestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            theme::apply_theme(ctx);
            self.theme_applied = true;
        }

        // Request focus on first frame
        if !self.input_focus_requested {
            self.input_focus_requested = true;
            self.select_all_countdown = 2;
            ctx.memory_mut(|m| {
                m.request_focus(egui::Id::new("ip_input_textedit"));
            });
        }

        // Apply select-all on startup or when user clicks the input
        if self.select_all_countdown > 0 {
            let id = egui::Id::new("ip_input_textedit");
            let n = self.address_input.len();
            if n > 0 {
                if let Some(mut state) = egui::widgets::text_edit::TextEditState::load(ctx, id) {
                    let end = Cursor { ccursor: egui::text::CCursor::new(n), ..Default::default() };
                    state.cursor.set_range(Some(CursorRange::two(Cursor::default(), end)));
                    state.store(ctx, id);
                }
            }
            self.select_all_countdown -= 1;
        }

        // Handle file drops
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(path) = i.raw.dropped_files[0].path.clone() {
                    self.handle_file_drop(path);
                }
            }
        });

        // Handle excel column picker confirmation
        if self.dialog_state.excel_column_confirmed {
            if let Some(col_idx) = self.dialog_state.selected_excel_column {
                if let Some((_, ref rows)) = self.imported_excel_data {
                    let rows_clone = rows.clone();
                    self.extract_ips_from_excel(&rows_clone, col_idx);
                    self.selected_ip_col = Some(col_idx);
                }
            }
            self.dialog_state.excel_column_confirmed = false;
            if self.is_running {
                self.refresh_ping();
            }
        }

        // Handle IP warning confirmation
        if self.dialog_state.ip_warning_confirmed {
            self.start_ping();
        }

        // Request repaint while running
        if self.is_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Dialogs
        dialogs::render_settings_dialog(ctx, &mut self.config, &mut self.dialog_state.show_settings);
        dialogs::render_ip_warning_dialog(ctx, &mut self.dialog_state);
        dialogs::render_excel_column_picker(ctx, &mut self.dialog_state);
        dialogs::render_error_dialog(ctx, &mut self.dialog_state);
        dialogs::render_about_dialog(ctx, &mut self.dialog_state.show_about);

        // Top panel - toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("PingTest").strong().color(theme::ACCENT).size(16.0));
                ui.separator();

                if self.is_running {
                    if ui.button(RichText::new("⏹ 停止").color(theme::FAIL_COLOR)).clicked() {
                        self.stop_ping();
                    }
                    if ui.button(RichText::new("🔄 刷新").color(theme::WARN_COLOR)).clicked() {
                        self.refresh_ping();
                    }
                } else if ui.button(RichText::new("▶ 开始").color(theme::SUCCESS_COLOR)).clicked() {
                    self.start_ping();
                }

                ui.separator();

                if ui.button("📂 导入").clicked() {
                    self.import_file();
                }

                if !self.targets.is_empty() {
                    if ui.button("📊 导出结果").clicked() {
                        self.export_results();
                    }

                    if self.imported_file_path.is_some() && ui.button("📎 插入到源表").clicked() {
                        self.export_to_source_excel();
                    }
                }

                ui.separator();

                if ui.button("⚙ 设置").clicked() {
                    self.dialog_state.show_settings = true;
                }

                if ui.button("ℹ 关于").clicked() {
                    self.dialog_state.show_about = true;
                }
            });
        });

        // Bottom panel - status bar
        egui::TopBottomPanel::bottom("status")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_height(32.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status_msg).color(theme::TEXT_DIM).size(12.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("© 2026 byff").color(theme::TEXT_DIM).size(10.0));
                        if !self.targets.is_empty() {
                            ui.separator();
                            let total = self.targets.len();
                            let alive = self.targets.iter()
                                .filter(|t| t.stats.read().is_alive)
                                .count();
                            ui.label(RichText::new(format!(
                                "在线: {}/{}", alive, total
                            )).color(if alive == total { theme::SUCCESS_COLOR } else { theme::WARN_COLOR }).size(12.0));
                        }
                    });
                });
            });

        // Left panel - address input
        egui::SidePanel::left("input_panel")
            .default_width(220.0)
            .min_width(160.0)
            .resizable(true)
            .show(ctx, |ui| {
                // Header row: centered with label + button
                ui.horizontal(|ui| {
                    ui.label(RichText::new("目标地址").strong().color(theme::ACCENT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add(egui::Button::new("整理IP").small().frame(true)).clicked() {
                            let cleaned = utils::extract_and_clean_ips(&self.address_input);
                            if !cleaned.is_empty() {
                                self.address_input = cleaned.clone();
                                self.last_cleaned_input = cleaned;
                                self.status_msg = "已整理IP地址".to_string();
                            } else {
                                self.status_msg = "未找到有效IP地址".to_string();
                            }
                        }
                    });
                });
                ui.label(RichText::new("支持混合文本，自动提取IP").color(theme::TEXT_DIM).size(10.0));
                ui.add_space(4.0);

                // Fill entire remaining panel height with the TextEdit
                let available = ui.available_size();
                let text_height = (available.y - 12.0).max(100.0);
                let _ = ui.allocate_ui(egui::vec2(available.x, text_height), |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.address_input)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("192.168.1.1\n10.0.0.0/24\nexample.com\n或粘贴含IP的任意文本")
                            .id(egui::Id::new("ip_input_textedit"))
                            .frame(true)
                    );
                    // Double-click TextEdit → select all (regardless of cursor position)
                    if response.clicked() {
                        let now = ctx.input(|i| i.time);
                        let is_double = self.last_input_click_secs
                            .map_or(false, |t| now - t < 0.3);
                        self.last_input_click_secs = Some(now);
                        if is_double && !self.address_input.is_empty() {
                            self.select_all_countdown = 2;
                        }
                    }
                });
            });

        // Central panel - results table
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.targets.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("输入IP地址并点击 ▶ 开始\n支持直接粘贴含IP的文本，自动提取")
                        .color(theme::TEXT_DIM)
                        .size(16.0));
                });
            } else {
                table::sort_targets(&mut self.sorted_indices, &self.targets, &self.table_state);
                table::render_table(ui, &self.targets, &self.sorted_indices, &mut self.table_state);
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.engine.read().stop();
        self.config.save();
    }
}
