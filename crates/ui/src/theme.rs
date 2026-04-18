//! Dark, low-noise theme tuned for CAD chrome.
//! Applied once at startup via egui::Context::set_visuals.

use egui::{Color32, Context, CornerRadius, FontDefinitions, Stroke, Visuals};

pub struct ThemeColors;

impl ThemeColors {
    pub const BG_DEEP: Color32 = Color32::from_rgb(0x14, 0x16, 0x1A);
    pub const BG_PANEL: Color32 = Color32::from_rgb(0x1B, 0x1E, 0x24);
    pub const BG_PANEL_ALT: Color32 = Color32::from_rgb(0x21, 0x25, 0x2C);
    pub const BG_HOVER: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x38);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(0x34, 0x3A, 0x45);
    pub const SEPARATOR: Color32 = Color32::from_rgb(0x2C, 0x30, 0x38);
    pub const TEXT: Color32 = Color32::from_rgb(0xDC, 0xDF, 0xE4);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8A, 0x91, 0x9C);
    pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xA3, 0xF7);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x2C, 0x5E, 0x93);
    pub const GRID_MAJOR: Color32 = Color32::from_rgb(0x2F, 0x35, 0x3F);
    pub const GRID_MINOR: Color32 = Color32::from_rgb(0x22, 0x26, 0x2D);
    pub const GRID_AXIS_X: Color32 = Color32::from_rgb(0xD0, 0x4B, 0x4B);
    pub const GRID_AXIS_Y: Color32 = Color32::from_rgb(0x4B, 0xC0, 0x6B);
}

pub fn apply_dark_theme(ctx: &Context) {
    install_phosphor_fonts(ctx);

    let mut visuals = Visuals::dark();

    visuals.panel_fill = ThemeColors::BG_PANEL;
    visuals.window_fill = ThemeColors::BG_PANEL;
    visuals.extreme_bg_color = ThemeColors::BG_DEEP;
    visuals.faint_bg_color = ThemeColors::BG_PANEL_ALT;
    visuals.code_bg_color = ThemeColors::BG_DEEP;

    visuals.override_text_color = Some(ThemeColors::TEXT);
    visuals.hyperlink_color = ThemeColors::ACCENT;
    visuals.selection.bg_fill = ThemeColors::ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, ThemeColors::ACCENT);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, ThemeColors::SEPARATOR);

    visuals.widgets.inactive.bg_fill = ThemeColors::BG_PANEL_ALT;
    visuals.widgets.inactive.weak_bg_fill = ThemeColors::BG_PANEL_ALT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, ThemeColors::SEPARATOR);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT);

    visuals.widgets.hovered.bg_fill = ThemeColors::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = ThemeColors::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ThemeColors::ACCENT_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT);

    visuals.widgets.active.bg_fill = ThemeColors::BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = ThemeColors::BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ThemeColors::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ThemeColors::TEXT);

    let radius = CornerRadius::same(3);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.window_corner_radius = CornerRadius::same(4);
    visuals.menu_corner_radius = CornerRadius::same(4);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(8);
    ctx.set_global_style(style);
}

fn install_phosphor_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}
