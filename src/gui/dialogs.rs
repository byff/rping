use egui::{RichText, Window, Context};
use crate::config::AppConfig;
use crate::gui::theme;

#[derive(Default)]
pub struct DialogState {
    pub show_settings: bool,
    pub show_ip_warning: bool,
    pub show_excel_column_picker: bool,
    pub show_about: bool,
    pub show_error: bool,
    pub error_title: String,
    pub error_message: String,
    pub ip_count_warning: usize,
    pub ip_warning_confirmed: bool,
    pub excel_columns: Vec<(usize, String)>,
    pub selected_excel_column: Option<usize>,
    pub excel_column_confirmed: bool,
}

impl DialogState {
    pub fn show_error(&mut self, title: &str, message: &str) {
        self.error_title = title.to_string();
        self.error_message = message.to_string();
        self.show_error = true;
    }
}

pub fn render_settings_dialog(ctx: &Context, config: &mut AppConfig, open: &mut bool) {
    if !*open {
        return;
    }

    let mut should_close = false;

    Window::new("⚙ 设置")
        .open(open)
        .resizable(true)
        .default_width(520.0)
        .show(ctx, |ui| {
            // === Ping 参数 ===
            ui.heading(RichText::new("Ping 参数").color(theme::ACCENT));
            ui.separator();
            ui.columns(2, |cols| {
                cols[0].horizontal(|ui| {
                    ui.label("超时(ms):");
                    let mut v = config.ping.timeout_ms as i64;
                    ui.add(egui::DragValue::new(&mut v).range(100..=30000).speed(100));
                    config.ping.timeout_ms = v.max(100) as u64;
                });
                cols[0].horizontal(|ui| {
                    ui.label("包大小(B):");
                    let mut v = config.ping.packet_size as i64;
                    ui.add(egui::DragValue::new(&mut v).range(1..=65500).speed(1));
                    config.ping.packet_size = v.max(1) as usize;
                });
                cols[1].horizontal(|ui| {
                    ui.label("间隔(ms):");
                    let mut v = config.ping.interval_ms as i64;
                    ui.add(egui::DragValue::new(&mut v).range(100..=60000).speed(100));
                    config.ping.interval_ms = v.max(100) as u64;
                });
                cols[1].horizontal(|ui| {
                    ui.label("并发数:");
                    let mut v = config.ping.max_concurrent as i64;
                    ui.add(egui::DragValue::new(&mut v).range(1..=2000).speed(10));
                    config.ping.max_concurrent = v.max(1) as usize;
                });
            });

            ui.add_space(10.0);

            // === 网络选项 ===
            ui.heading(RichText::new("网络选项").color(theme::ACCENT));
            ui.separator();
            ui.checkbox(&mut config.cidr_strip_first_last, "CIDR 去掉首尾IP（网络地址和广播地址）");

            ui.add_space(10.0);

            // === 导出选项 ===
            ui.heading(RichText::new("导出字段").color(theme::ACCENT));
            ui.separator();
            ui.columns(2, |cols| {
                cols[0].checkbox(&mut config.export.export_ip, "IP地址");
                cols[0].checkbox(&mut config.export.export_hostname, "主机名");
                cols[0].checkbox(&mut config.export.export_success_count, "成功次数");
                cols[0].checkbox(&mut config.export.export_fail_count, "失败次数");
                cols[0].checkbox(&mut config.export.export_fail_rate, "失败率");
                cols[1].checkbox(&mut config.export.export_total_sent, "总发送数");
                cols[1].checkbox(&mut config.export.export_last_rtt, "当前延迟");
                cols[1].checkbox(&mut config.export.export_max_rtt, "最大延迟");
                cols[1].checkbox(&mut config.export.export_min_rtt, "最小延迟");
                cols[1].checkbox(&mut config.export.export_avg_rtt, "平均延迟");
            });

            ui.add_space(10.0);

            // === 其他 ===
            ui.heading(RichText::new("其他").color(theme::ACCENT));
            ui.separator();
            ui.columns(2, |cols| {
                cols[0].checkbox(&mut config.remember_addresses, "记忆地址列表");
                cols[1].checkbox(&mut config.debug_mode, "调试模式（需重启）");
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button(RichText::new("💾 保存").color(theme::ACCENT)).clicked() {
                    config.save();
                }
                if ui.button("关闭").clicked() {
                    should_close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(RichText::new("恢复默认").color(theme::TEXT_DIM)).clicked() {
                        *config = AppConfig::default();
                    }
                });
            });
        });

    if should_close {
        *open = false;
    }
}

pub fn render_ip_warning_dialog(ctx: &Context, state: &mut DialogState) {
    if !state.show_ip_warning {
        return;
    }

    Window::new("⚠ IP数量警告")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(RichText::new(format!(
                "即将展开 {} 个IP地址。\n建议不要超过1000个IP，可能影响性能。\n\n是否继续？",
                state.ip_count_warning
            )).size(14.0));

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("继续").color(theme::WARN_COLOR)).clicked() {
                    state.ip_warning_confirmed = true;
                    state.show_ip_warning = false;
                }
                if ui.button("取消").clicked() {
                    state.ip_warning_confirmed = false;
                    state.show_ip_warning = false;
                }
            });
        });
}

pub fn render_excel_column_picker(ctx: &Context, state: &mut DialogState) {
    if !state.show_excel_column_picker {
        return;
    }

    Window::new("📊 选择IP列")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(RichText::new("检测到多个可能包含IP的列，请选择:").size(14.0));
            ui.add_space(8.0);

            for (idx, name) in &state.excel_columns {
                let selected = state.selected_excel_column == Some(*idx);
                if ui.selectable_label(selected, format!("列 {} - {}", idx + 1, name)).clicked() {
                    state.selected_excel_column = Some(*idx);
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("确定").color(theme::ACCENT)).clicked() {
                    if state.selected_excel_column.is_some() {
                        state.excel_column_confirmed = true;
                        state.show_excel_column_picker = false;
                    }
                }
                if ui.button("取消").clicked() {
                    state.selected_excel_column = None;
                    state.excel_column_confirmed = false;
                    state.show_excel_column_picker = false;
                }
            });
        });
}

pub fn render_error_dialog(ctx: &Context, state: &mut DialogState) {
    if !state.show_error {
        return;
    }

    Window::new(format!("❌ {}", state.error_title))
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(&state.error_message).size(14.0));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("确定").color(theme::ACCENT)).clicked() {
                    state.show_error = false;
                }
            });
        });
}

pub fn render_about_dialog(ctx: &Context, open: &mut bool) {
    Window::new("关于 PingTest")
        .open(open)
        .resizable(false)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("PingTest").size(24.0).color(theme::ACCENT).strong());
                ui.label(RichText::new("v0.2.1").color(theme::TEXT_DIM));
                ui.add_space(8.0);
                ui.label("高性能多目标 Ping 工具");
                ui.add_space(4.0);
                ui.label(RichText::new("Powered by byff").color(theme::ACCENT).size(12.0));
                ui.add_space(4.0);
                ui.label(RichText::new("© 2026 byff. All rights reserved.").color(theme::TEXT_DIM).size(10.0));
                ui.label(RichText::new("Built with Rust + egui").color(theme::TEXT_DIM).size(10.0));
            });
        });
}
