use egui::{RichText, Window, Context};
use crate::config::AppConfig;
use crate::gui::theme;

#[derive(Default)]
pub struct DialogState {
    pub show_settings: bool,
    pub show_ip_warning: bool,
    pub show_excel_column_picker: bool,
    pub show_about: bool,
    pub ip_count_warning: usize,
    pub ip_warning_confirmed: bool,
    pub excel_columns: Vec<(usize, String)>,
    pub selected_excel_column: Option<usize>,
    pub excel_column_confirmed: bool,
}

pub fn render_settings_dialog(ctx: &Context, config: &mut AppConfig, open: &mut bool) {
    Window::new("⚙ 设置")
        .open(open)
        .resizable(false)
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.heading(RichText::new("Ping 参数").color(theme::ACCENT));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("超时时间(ms):");
                let mut timeout = config.ping.timeout_ms as i64;
                ui.add(egui::DragValue::new(&mut timeout).range(100..=30000).speed(100));
                config.ping.timeout_ms = timeout.max(100) as u64;
            });

            ui.horizontal(|ui| {
                ui.label("数据包大小(bytes):");
                let mut size = config.ping.packet_size as i64;
                ui.add(egui::DragValue::new(&mut size).range(1..=65500).speed(1));
                config.ping.packet_size = size.max(1) as usize;
            });

            ui.horizontal(|ui| {
                ui.label("Ping间隔(ms):");
                let mut interval = config.ping.interval_ms as i64;
                ui.add(egui::DragValue::new(&mut interval).range(100..=60000).speed(100));
                config.ping.interval_ms = interval.max(100) as u64;
            });

            ui.horizontal(|ui| {
                ui.label("最大并发数:");
                let mut max = config.ping.max_concurrent as i64;
                ui.add(egui::DragValue::new(&mut max).range(1..=2000).speed(10));
                config.ping.max_concurrent = max.max(1) as usize;
            });

            ui.add_space(12.0);
            ui.heading(RichText::new("导出选项").color(theme::ACCENT));
            ui.separator();

            ui.checkbox(&mut config.export.export_hostname, "主机名");
            ui.checkbox(&mut config.export.export_ip, "IP地址");
            ui.checkbox(&mut config.export.export_success_count, "成功次数");
            ui.checkbox(&mut config.export.export_fail_count, "失败次数");
            ui.checkbox(&mut config.export.export_fail_rate, "失败率");
            ui.checkbox(&mut config.export.export_total_sent, "总发送数");
            ui.checkbox(&mut config.export.export_last_rtt, "当前延迟");
            ui.checkbox(&mut config.export.export_max_rtt, "最大延迟");
            ui.checkbox(&mut config.export.export_min_rtt, "最小延迟");
            ui.checkbox(&mut config.export.export_avg_rtt, "平均延迟");

            ui.add_space(12.0);
            ui.heading(RichText::new("其他").color(theme::ACCENT));
            ui.separator();
            ui.checkbox(&mut config.remember_addresses, "记忆地址列表");

            ui.add_space(8.0);
            if ui.button("保存配置").clicked() {
                config.save();
            }
        });
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

pub fn render_about_dialog(ctx: &Context, open: &mut bool) {
    Window::new("关于 RPing")
        .open(open)
        .resizable(false)
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("RPing").size(24.0).color(theme::ACCENT).strong());
                ui.label(RichText::new("v0.1.0").color(theme::TEXT_DIM));
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
