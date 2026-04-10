use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use eframe::egui;
use egui::{RichText, Layout, Align};

use crate::config::AppConfig;
use crate::ping::{PingEngine, PingTarget, PingStats};
use crate::gui::{table, dialogs, theme};
use crate::utils;
use crate::excel;

pub struct RPingApp {
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

    // Runtime
    runtime: tokio::runtime::Runtime,

    // Theme applied
    theme_applied: bool,
}

impl RPingApp {
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

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        Self {
            config,
            engine: Arc::new(RwLock::new(engine)),
            targets: Vec::new(),
            sorted_indices: Vec::new(),
            table_state: table::TableState::default(),
            dialog_state: dialogs::DialogState::default(),
            address_input,
            is_running: false,
            imported_file_path: None,
            imported_excel_data: None,
            selected_ip_col: None,
            status_msg: "就绪".to_string(),
            runtime,
            theme_applied: false,
        }
    }

    fn start_ping(&mut self) {
        if self.address_input.trim().is_empty() {
            self.status_msg = "请输入IP地址".to_string();
            return;
        }

        // Check IP count
        let count = utils::count_cidr_ips(&self.address_input);
        if count > 1000 && !self.dialog_state.ip_warning_confirmed {
            self.dialog_state.ip_count_warning = count;
            self.dialog_state.show_ip_warning = true;
            return;
        }

        let parsed = utils::parse_targets(&self.address_input);
        if parsed.is_empty() {
            self.status_msg = "未解析到有效IP地址".to_string();
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

        // Save addresses if remember is on
        if self.config.remember_addresses {
            self.config.last_addresses = self.address_input
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            self.config.save();
        }

        // Create and start engine
        let mut engine = PingEngine::new(
            self.config.ping.timeout_ms,
            self.config.ping.interval_ms,
            self.config.ping.packet_size,
            self.config.ping.max_concurrent,
        );
        engine.set_targets(self.targets.clone());
        engine.start(&self.runtime.handle());
        *self.engine.write() = engine;

        self.is_running = true;
        self.status_msg = format!("正在 Ping {} 个目标...", self.targets.len());
    }

    fn stop_ping(&mut self) {
        self.engine.read().stop();
        self.is_running = false;
        self.status_msg = "已停止".to_string();
    }

    fn handle_file_drop(&mut self, path: PathBuf) {
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Remember directory
        if let Some(dir) = path.parent() {
            self.config.last_import_dir = Some(dir.to_path_buf());
            self.config.save();
        }

        match ext.as_str() {
            "txt" | "csv" => {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    self.address_input = content;
                    self.status_msg = format!("已导入: {}", path.display());
                }
            }
            "xlsx" | "xls" => {
                match excel::read_excel(&path) {
                    Ok((headers, rows)) => {
                        let ip_cols = utils::find_ip_columns(&headers, &rows);
                        if ip_cols.is_empty() {
                            self.status_msg = "未在Excel中找到IP列".to_string();
                        } else if ip_cols.len() == 1 {
                            // Auto-select single IP column
                            let col_idx = ip_cols[0].0;
                            self.extract_ips_from_excel(&rows, col_idx);
                            self.imported_file_path = Some(path);
                            self.imported_excel_data = Some((headers, rows));
                            self.selected_ip_col = Some(col_idx);
                        } else {
                            // Multiple columns, let user pick
                            self.dialog_state.excel_columns = ip_cols;
                            self.dialog_state.show_excel_column_picker = true;
                            self.imported_file_path = Some(path);
                            self.imported_excel_data = Some((headers, rows));
                        }
                    }
                    Err(e) => {
                        self.status_msg = format!("读取Excel失败: {}", e);
                    }
                }
            }
            _ => {
                self.status_msg = format!("不支持的文件格式: {}", ext);
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
        self.status_msg = format!("已从Excel导入 {} 个地址", ips.len());
    }

    fn import_file(&mut self) {
        let initial_dir = self.config.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("所有支持格式", &["txt", "csv", "xlsx", "xls"])
            .add_filter("文本文件", &["txt", "csv"])
            .add_filter("Excel文件", &["xlsx", "xls"]);

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.pick_file() {
            self.handle_file_drop(path);
        }
    }

    fn export_results(&self) {
        let initial_dir = self.config.last_import_dir.clone();
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Excel文件", &["xlsx"])
            .set_file_name("ping_results.xlsx");

        if let Some(dir) = initial_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
            match excel::export_results(&path, &self.targets, &self.config.export) {
                Ok(_) => tracing::info!("导出成功: {}", path.display()),
                Err(e) => tracing::error!("导出失败: {}", e),
            }
        }
    }

    fn export_to_source_excel(&self) {
        if let (Some(source_path), Some((_, _)), Some(ip_col)) = (
            &self.imported_file_path,
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
                    Ok(_) => tracing::info!("插入结果成功: {}", path.display()),
                    Err(e) => tracing::error!("插入结果失败: {}", e),
                }
            }
        }
    }
}

impl eframe::App for RPingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            theme::apply_theme(ctx);
            self.theme_applied = true;
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
        dialogs::render_about_dialog(ctx, &mut self.dialog_state.show_about);

        // Top panel - toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("RPing").strong().color(theme::ACCENT).size(16.0));
                ui.separator();

                if self.is_running {
                    if ui.button(RichText::new("⏹ 停止").color(theme::FAIL_COLOR)).clicked() {
                        self.stop_ping();
                    }
                } else {
                    if ui.button(RichText::new("▶ 开始").color(theme::SUCCESS_COLOR)).clicked() {
                        self.start_ping();
                    }
                }

                ui.separator();

                if ui.button("📂 导入").clicked() {
                    self.import_file();
                }

                if !self.targets.is_empty() {
                    if ui.button("📊 导出结果").clicked() {
                        self.export_results();
                    }

                    if self.imported_file_path.is_some() {
                        if ui.button("📎 插入到源表").clicked() {
                            self.export_to_source_excel();
                        }
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
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status_msg).color(theme::TEXT_DIM).size(11.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !self.targets.is_empty() {
                        let total = self.targets.len();
                        let alive = self.targets.iter()
                            .filter(|t| t.stats.read().is_alive)
                            .count();
                        ui.label(RichText::new(format!(
                            "在线: {}/{}", alive, total
                        )).color(if alive == total { theme::SUCCESS_COLOR } else { theme::WARN_COLOR }).size(11.0));
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
                ui.label(RichText::new("目标地址").strong().color(theme::ACCENT));
                ui.label(RichText::new("每行一个IP/域名/CIDR").color(theme::TEXT_DIM).size(10.0));
                ui.add_space(4.0);

                let available = ui.available_size();
                egui::ScrollArea::vertical()
                    .max_height(available.y - 8.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.address_input)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("192.168.1.1\n10.0.0.0/24\nexample.com")
                        );
                    });
            });

        // Central panel - results table
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.targets.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("输入IP地址并点击 ▶ 开始")
                        .color(theme::TEXT_DIM)
                        .size(16.0));
                });
            } else {
                // Re-sort
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
