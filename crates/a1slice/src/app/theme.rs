//! Colours, the sunset stripe, and the dark window theme.

use eframe::egui::{self, Color32, Stroke};

const CANVAS: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
pub(super) const CREAM: Color32 = Color32::from_rgb(0xf4, 0xf1, 0xea);
pub(super) const CREAM_HEAD: Color32 = Color32::from_rgb(0xff, 0xf8, 0xe0);
pub(super) const MUTED: Color32 = Color32::from_rgb(0xa8, 0xa8, 0xa8);
pub(super) const ORANGE: Color32 = Color32::from_rgb(0xfa, 0x52, 0x0f);
pub(super) const SURFACE: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1e);
pub(super) const INK: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x1f);
pub(super) const BEIGE: Color32 = Color32::from_rgb(0xe6, 0xd5, 0xa8);
pub(super) const HAIRLINE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
pub(super) const HAIRLINE_HOVER: Color32 = Color32::from_rgb(0xc7, 0xc7, 0xc7);
pub(super) const HOLD_ARM_SECS: f64 = 0.280;
pub(super) const HOLD_STEP_SECS: f64 = 0.340;

pub(super) fn apply_theme(ctx: &egui::Context) {
    install_type(ctx);
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = CANVAS;
    visuals.panel_fill = CANVAS;
    visuals.extreme_bg_color = Color32::from_rgb(0x14, 0x14, 0x14);
    visuals.faint_bg_color = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, CREAM);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x1c, 0x18);
    visuals.selection.bg_fill = ORANGE;
    visuals.hyperlink_color = ORANGE;
    visuals.widgets.active.bg_fill = ORANGE;
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        // A floating bar paints over the last letters. A solid bar keeps its own column.
        style.spacing.scroll = egui::style::ScrollStyle::solid();
        style.visuals.override_text_color = Some(CREAM);
    });
}

fn install_type(ctx: &egui::Context) {
    let sans = "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf";
    let serif = "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf";
    if let Ok(bytes) = std::fs::read(sans) {
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "Noto Sans",
            egui::FontData::from_owned(bytes),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
    }
    if let Ok(bytes) = std::fs::read(serif) {
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "Noto Serif",
            egui::FontData::from_owned(bytes),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Name("Noto Serif".into()),
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
    }
}
