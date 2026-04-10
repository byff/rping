use egui::{Color32, Visuals, Style, Rounding, Stroke};

pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Modern dark tech theme
    visuals.window_fill = Color32::from_rgb(24, 26, 32);
    visuals.panel_fill = Color32::from_rgb(24, 26, 32);
    visuals.faint_bg_color = Color32::from_rgb(32, 35, 42);
    visuals.extreme_bg_color = Color32::from_rgb(18, 20, 24);

    // Accent color - tech blue
    let accent = Color32::from_rgb(64, 156, 255);

    visuals.selection.bg_fill = accent.linear_multiply(0.3);
    visuals.selection.stroke = Stroke::new(1.0, accent);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(32, 35, 42);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(180, 185, 195));
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 44, 52);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(160, 165, 175));
    visuals.widgets.inactive.rounding = Rounding::same(4.0);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 55, 65);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 225, 235));
    visuals.widgets.hovered.rounding = Rounding::same(4.0);

    visuals.widgets.active.bg_fill = accent.linear_multiply(0.4);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(4.0);

    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(45, 48, 56));

    let mut style = Style::default();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(12.0, 4.0);

    ctx.set_style(style);
}

pub const ACCENT: Color32 = Color32::from_rgb(64, 156, 255);
pub const SUCCESS_COLOR: Color32 = Color32::from_rgb(80, 200, 120);
pub const FAIL_COLOR: Color32 = Color32::from_rgb(255, 85, 85);
pub const WARN_COLOR: Color32 = Color32::from_rgb(255, 180, 50);
pub const TEXT_DIM: Color32 = Color32::from_rgb(120, 125, 135);
