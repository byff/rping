use egui::{Ui, RichText, Color32};
use crate::ping::PingTarget;
use crate::gui::theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortColumn {
    Index,
    Hostname,
    Ip,
    SuccessCount,
    FailCount,
    FailRate,
    TotalSent,
    LastRtt,
    MaxRtt,
    MinRtt,
    AvgRtt,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub struct TableState {
    pub sort_column: SortColumn,
    pub sort_order: SortOrder,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            sort_column: SortColumn::Index,
            sort_order: SortOrder::Ascending,
        }
    }
}

impl TableState {
    pub fn toggle_sort(&mut self, col: SortColumn) {
        if self.sort_column == col {
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            self.sort_column = col;
            self.sort_order = SortOrder::Ascending;
        }
    }

    pub fn sort_indicator(&self, col: SortColumn) -> &str {
        if self.sort_column == col {
            match self.sort_order {
                SortOrder::Ascending => " ▲",
                SortOrder::Descending => " ▼",
            }
        } else {
            ""
        }
    }
}

pub fn sort_targets(targets: &mut [usize], all_targets: &[PingTarget], state: &TableState) {
    targets.sort_by(|&a, &b| {
        let ta = &all_targets[a];
        let tb = &all_targets[b];
        let sa = ta.stats.read();
        let sb = tb.stats.read();

        let cmp = match state.sort_column {
            SortColumn::Index => ta.index.cmp(&tb.index),
            SortColumn::Hostname => ta.hostname.cmp(&tb.hostname),
            SortColumn::Ip => ta.ip.cmp(&tb.ip),
            SortColumn::SuccessCount => sa.success_count.cmp(&sb.success_count),
            SortColumn::FailCount => sa.fail_count.cmp(&sb.fail_count),
            SortColumn::FailRate => sa.fail_rate().partial_cmp(&sb.fail_rate()).unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::TotalSent => sa.total_sent.cmp(&sb.total_sent),
            SortColumn::LastRtt => sa.last_rtt_us.cmp(&sb.last_rtt_us),
            SortColumn::MaxRtt => sa.max_rtt_us.cmp(&sb.max_rtt_us),
            SortColumn::MinRtt => sa.min_rtt_us.cmp(&sb.min_rtt_us),
            SortColumn::AvgRtt => sa.avg_rtt_us().cmp(&sb.avg_rtt_us()),
        };

        match state.sort_order {
            SortOrder::Ascending => cmp,
            SortOrder::Descending => cmp.reverse(),
        }
    });
}

pub fn render_table(ui: &mut Ui, targets: &[PingTarget], sorted_indices: &[usize], table_state: &mut TableState) {
    use egui_extras::{TableBuilder, Column};

    let available = ui.available_size();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .min_scrolled_height(0.0)
        .max_scroll_height(available.y)
        .column(Column::initial(40.0).at_least(30.0))   // #
        .column(Column::initial(140.0).at_least(80.0))  // Hostname
        .column(Column::initial(120.0).at_least(80.0))  // IP
        .column(Column::initial(65.0).at_least(45.0))   // Success
        .column(Column::initial(65.0).at_least(45.0))   // Fail
        .column(Column::initial(70.0).at_least(50.0))   // Fail%
        .column(Column::initial(65.0).at_least(45.0))   // Total
        .column(Column::initial(75.0).at_least(55.0))   // RTT
        .column(Column::initial(75.0).at_least(55.0))   // Max
        .column(Column::initial(75.0).at_least(55.0))   // Min
        .column(Column::remainder().at_least(55.0))      // Avg
        .header(22.0, |mut header| {
            let cols = [
                (SortColumn::Index, "序号"),
                (SortColumn::Hostname, "主机名"),
                (SortColumn::Ip, "IP地址"),
                (SortColumn::SuccessCount, "成功"),
                (SortColumn::FailCount, "失败"),
                (SortColumn::FailRate, "失败率"),
                (SortColumn::TotalSent, "总计"),
                (SortColumn::LastRtt, "延迟"),
                (SortColumn::MaxRtt, "最大"),
                (SortColumn::MinRtt, "最小"),
                (SortColumn::AvgRtt, "平均"),
            ];
            for (col, name) in cols {
                header.col(|ui| {
                    let label = format!("{}{}", name, table_state.sort_indicator(col));
                    if ui.selectable_label(table_state.sort_column == col,
                        RichText::new(label).strong().color(theme::ACCENT).size(12.0)
                    ).clicked() {
                        table_state.toggle_sort(col);
                    }
                });
            }
        })
        .body(|body| {
            body.rows(20.0, sorted_indices.len(), |mut row| {
                let idx = sorted_indices[row.index()];
                let target = &targets[idx];
                let stats = target.stats.read();

                // Index
                row.col(|ui| {
                    ui.label(RichText::new(format!("{}", target.index + 1)).color(theme::TEXT_DIM).size(11.0));
                });
                // Hostname
                row.col(|ui| {
                    ui.label(RichText::new(&target.hostname).size(11.0));
                });
                // IP
                row.col(|ui| {
                    let color = if stats.is_alive { theme::SUCCESS_COLOR } else if stats.total_sent > 0 { theme::FAIL_COLOR } else { Color32::from_rgb(180, 185, 195) };
                    ui.label(RichText::new(target.ip.to_string()).color(color).size(11.0));
                });
                // Success
                row.col(|ui| {
                    ui.label(RichText::new(format!("{}", stats.success_count)).color(theme::SUCCESS_COLOR).size(11.0));
                });
                // Fail
                row.col(|ui| {
                    let color = if stats.fail_count > 0 { theme::FAIL_COLOR } else { theme::TEXT_DIM };
                    ui.label(RichText::new(format!("{}", stats.fail_count)).color(color).size(11.0));
                });
                // Fail rate
                row.col(|ui| {
                    let rate = stats.fail_rate();
                    let color = if rate > 50.0 { theme::FAIL_COLOR } else if rate > 10.0 { theme::WARN_COLOR } else { theme::SUCCESS_COLOR };
                    ui.label(RichText::new(format!("{:.1}%", rate)).color(color).size(11.0));
                });
                // Total
                row.col(|ui| {
                    ui.label(RichText::new(format!("{}", stats.total_sent)).size(11.0));
                });
                // Last RTT
                row.col(|ui| {
                    let text = match stats.last_rtt_us {
                        Some(us) => format!("{:.1}ms", us as f64 / 1000.0),
                        None => "-".to_string(),
                    };
                    ui.label(RichText::new(text).size(11.0));
                });
                // Max RTT
                row.col(|ui| {
                    let text = if stats.success_count > 0 { format!("{:.1}ms", stats.max_rtt_us as f64 / 1000.0) } else { "-".to_string() };
                    ui.label(RichText::new(text).size(11.0));
                });
                // Min RTT
                row.col(|ui| {
                    let text = if stats.success_count > 0 { format!("{:.1}ms", stats.min_rtt_us as f64 / 1000.0) } else { "-".to_string() };
                    ui.label(RichText::new(text).size(11.0));
                });
                // Avg RTT
                row.col(|ui| {
                    let text = if stats.success_count > 0 { format!("{:.1}ms", stats.avg_rtt_us() as f64 / 1000.0) } else { "-".to_string() };
                    ui.label(RichText::new(text).size(11.0));
                });
            });
        });
}
