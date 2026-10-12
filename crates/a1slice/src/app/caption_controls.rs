//! Color, place, size, and font controls for the captions dock.

use a1slice_core::captions::{
    caption_ink, caption_metrics, clamp_caption_font_size, style_with_color, style_with_font_size,
    style_with_preset_size, CAPTION_PALETTE, MAX_CAPTION_FONT_SIZE, MIN_CAPTION_FONT_SIZE,
};
use a1slice_core::types::*;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::support::normalize_hex;
use super::theme::{HAIRLINE, HAIRLINE_HOVER, MUTED, ORANGE};
use super::widgets::choice;

/// Space between buttons in one group. The buttons themselves stay 44px tall.
pub(super) const CHIP_GAP: f32 = 8.0;
/// Space between Place, Size, and Font, so they read as three groups.
const GROUP_GAP: f32 = 40.0;
/// Space under a group label, tighter than the space between groups.
const LABEL_GAP: f32 = 8.0;
pub(super) const SECTION_GAP: f32 = 20.0;

pub(super) fn color_row(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    let current = caption_ink(style).hex;
    labeled_group(ui, "Color", 32.0, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(CHIP_GAP, CHIP_GAP);
        for item in CAPTION_PALETTE {
            if swatch(
                ui,
                item.hex,
                item.label,
                current.eq_ignore_ascii_case(item.hex),
            )
            .clicked()
            {
                *style = style_with_color(style, item.hex);
                ui.ctx()
                    .data_mut(|data| data.remove::<String>(color_draft_id()));
                changed = true;
            }
        }
        changed |= color_field(ui, style);
        changed
    });
    changed
}

pub(super) fn style_row(ui: &mut egui::Ui, style: &mut CaptionStyle, wide: bool) -> bool {
    arrange_style(ui, style, wide).0
}

fn arrange_style(
    ui: &mut egui::Ui,
    style: &mut CaptionStyle,
    wide: bool,
) -> (bool, [egui::Rect; 3]) {
    let mut changed = false;
    let mut rects = [egui::Rect::NOTHING; 3];
    if wide {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = GROUP_GAP;
            let (place_changed, place) =
                labeled_group(ui, "Place", 44.0, |ui| place_chips(ui, style));
            let (size_changed, size) = labeled_group(ui, "Size", 44.0, |ui| size_chips(ui, style));
            let (font_changed, font) = labeled_group(ui, "Font", 44.0, |ui| font_chips(ui, style));
            changed = place_changed || size_changed || font_changed;
            rects = [place, size, font];
        });
    } else {
        let (place_changed, place) = labeled_group(ui, "Place", 44.0, |ui| place_chips(ui, style));
        ui.add_space(SECTION_GAP);
        let (size_changed, size) = labeled_group(ui, "Size", 44.0, |ui| size_chips(ui, style));
        ui.add_space(SECTION_GAP);
        let (font_changed, font) = labeled_group(ui, "Font", 44.0, |ui| font_chips(ui, style));
        changed = place_changed || size_changed || font_changed;
        rects = [place, size, font];
    }
    (changed, rects)
}

/// Label above the buttons. The group stays as wide as its buttons, not the whole row.
fn labeled_group(
    ui: &mut egui::Ui,
    label: &str,
    row_h: f32,
    add: impl FnOnce(&mut egui::Ui) -> bool,
) -> (bool, egui::Rect) {
    let mut changed = false;
    let response = ui.allocate_ui_with_layout(
        egui::Vec2::ZERO,
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.spacing_mut().item_spacing.y = LABEL_GAP;
            row_label(ui, label);
            ui.allocate_ui_with_layout(
                egui::vec2(0.0, row_h),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = CHIP_GAP;
                    changed = add(ui);
                },
            );
        },
    );
    (changed, response.response.rect)
}

fn place_chips(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    for (position, label) in [
        (CaptionPosition::Bottom, "Bottom"),
        (CaptionPosition::Middle, "Middle"),
        (CaptionPosition::Top, "Top"),
    ] {
        if choice(ui, label, style.position == position).clicked() {
            style.position = position;
            changed = true;
        }
    }
    changed
}

fn size_chips(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    for (size, label) in [
        (CaptionSize::Small, "Small"),
        (CaptionSize::Medium, "Medium"),
        (CaptionSize::Large, "Large"),
    ] {
        let selected = style.font_size.is_none() && style.size == size;
        if choice(ui, label, selected).clicked() {
            *style = style_with_preset_size(style, size);
            ui.ctx()
                .data_mut(|data| data.remove::<String>(size_draft_id()));
            changed = true;
        }
    }
    changed |= size_field(ui, style);
    changed
}

fn font_chips(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    for (font, label) in [
        (CaptionFont::Sans, "Sans"),
        (CaptionFont::Serif, "Serif"),
        (CaptionFont::Mono, "Mono"),
    ] {
        if choice(ui, label, style.font == font).clicked() {
            style.font = font;
            changed = true;
        }
    }
    changed
}

pub(super) fn row_label(ui: &mut egui::Ui, text: &str) {
    // A shrink-wrapped group starts with no width. A wrapping label then
    // stacks one letter per line, so the word stays on one line.
    let _ = ui.add(
        egui::Label::new(RichText::new(text).size(14.0).color(MUTED))
            .selectable(false)
            .wrap_mode(egui::TextWrapMode::Extend),
    );
}

fn swatch(ui: &mut egui::Ui, hex: &str, label: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    let fill = hex_color(hex);
    let border = if selected {
        ORANGE
    } else if response.hovered() {
        HAIRLINE_HOVER
    } else if fill == Color32::WHITE || fill == Color32::from_rgb(0xff, 0xf8, 0xe0) {
        Color32::from_rgb(0x3a, 0x3a, 0x3a)
    } else {
        HAIRLINE
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(8),
        fill,
        Stroke::new(if selected { 2.0_f32 } else { 1.0_f32 }, border),
        egui::StrokeKind::Inside,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.on_hover_text(label)
}

fn size_field(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let id = size_draft_id();
    let shown = caption_metrics(style).font_size.to_string();
    let mut text = ui
        .ctx()
        .data(|data| data.get_temp::<String>(id))
        .unwrap_or(shown);
    let response = text_field(ui, &mut text, 56.0, 44.0, "");
    let mut changed = false;
    if response.changed() {
        text = text
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(2)
            .collect();
        ui.ctx().data_mut(|data| data.insert_temp(id, text.clone()));
        if let Ok(px) = text.parse::<i64>() {
            if (MIN_CAPTION_FONT_SIZE..=MAX_CAPTION_FONT_SIZE).contains(&px) {
                let next = style_with_font_size(style, px as f64);
                if next != *style {
                    *style = next;
                    changed = true;
                }
            }
        }
    }
    if response.lost_focus() {
        let px = text
            .parse::<f64>()
            .map(clamp_caption_font_size)
            .unwrap_or(caption_metrics(style).font_size);
        let next = style_with_font_size(style, px as f64);
        changed |= next != *style;
        *style = next;
        ui.ctx().data_mut(|data| data.remove::<String>(id));
    }
    changed
}

fn size_draft_id() -> egui::Id {
    egui::Id::new("a1-caption-size")
}

fn color_field(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let id = color_draft_id();
    let shown = caption_ink(style).hex;
    let mut text = ui
        .ctx()
        .data(|data| data.get_temp::<String>(id))
        .unwrap_or(shown);
    let response = text_field(ui, &mut text, 96.0, 32.0, "#ffb83e");
    let mut changed = false;
    if response.changed() {
        ui.ctx().data_mut(|data| data.insert_temp(id, text.clone()));
        let trimmed = text.trim();
        if trimmed.is_empty() {
            changed = style.custom_color.take().is_some();
        } else if let Some(hex) = normalize_hex(trimmed) {
            let next = style_with_color(style, &hex);
            if next != *style {
                *style = next;
                changed = true;
            }
        }
    }
    if response.lost_focus() {
        ui.ctx().data_mut(|data| data.remove::<String>(id));
    }
    changed
}

fn color_draft_id() -> egui::Id {
    egui::Id::new("a1-caption-color")
}

fn text_field(
    ui: &mut egui::Ui,
    text: &mut String,
    width: f32,
    height: f32,
    hint: &str,
) -> egui::Response {
    ui.scope(|ui| {
        {
            let look = ui.style_mut();
            let input = Color32::from_rgb(0x14, 0x14, 0x14);
            let edge = Color32::from_rgb(0x3a, 0x3a, 0x3a);
            for visuals in [
                &mut look.visuals.widgets.inactive,
                &mut look.visuals.widgets.hovered,
                &mut look.visuals.widgets.active,
            ] {
                visuals.corner_radius = egui::CornerRadius::same(8);
                visuals.bg_fill = input;
            }
            look.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, edge);
            look.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ORANGE);
            look.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ORANGE);
            look.visuals.extreme_bg_color = input;
        }
        let font = if height >= 40.0 { 16.0 } else { 14.0 };
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(width)
                .min_size(egui::vec2(width, height))
                .margin(egui::Margin::symmetric(8, 0))
                .horizontal_align(egui::Align::Center)
                .vertical_align(egui::Align::Center)
                .hint_text(hint)
                .font(egui::FontId::proportional(font)),
        )
    })
    .inner
}

fn hex_color(hex: &str) -> Color32 {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Color32::WHITE;
    }
    let channel = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).unwrap_or(255);
    Color32::from_rgb(channel(0), channel(2), channel(4))
}

#[cfg(test)]
mod tests {
    use super::arrange_style;
    use a1slice_core::types::*;

    fn sample_style() -> CaptionStyle {
        CaptionStyle {
            color: CaptionColor::White,
            custom_color: None,
            position: CaptionPosition::Bottom,
            size: CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        }
    }

    fn show_style(wide: bool, width: f32) -> [egui::Rect; 3] {
        let ctx = egui::Context::default();
        let mut style = sample_style();
        let mut rects = [egui::Rect::NOTHING; 3];
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width + 80.0, 800.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_max_width(width);
                    rects = arrange_style(ui, &mut style, wide).1;
                });
            },
        );
        rects
    }

    #[test]
    fn wide_groups_sit_apart_on_one_row() {
        let [place, size, font] = show_style(true, 1000.0);
        assert!(
            place.height() >= 60.0 && place.height() < 90.0,
            "the label is one line above the 44px buttons, got {}",
            place.height()
        );
        assert!(
            place.width() > 200.0 && size.width() > 240.0 && font.width() > 180.0,
            "each group is as wide as its buttons, place {} size {} font {}",
            place.width(),
            size.width(),
            font.width()
        );
        assert!(
            size.left() >= place.right() + 32.0,
            "size should sit apart from place, place {} size {}",
            place.right(),
            size.left()
        );
        assert!(
            font.left() >= size.right() + 32.0,
            "font should sit apart from size, size {} font {}",
            size.right(),
            font.left()
        );
        assert!(
            font.right() <= 1008.0,
            "the three groups stay inside the dock, font ends at {}",
            font.right()
        );
        assert!(
            (place.top() - size.top()).abs() < 2.0 && (size.top() - font.top()).abs() < 2.0,
            "the groups share the row"
        );
    }

    #[test]
    fn narrow_groups_stack_with_air_between_them() {
        let [place, size, font] = show_style(false, 420.0);
        assert!(size.top() >= place.bottom() + 16.0);
        assert!(font.top() >= size.bottom() + 16.0);
        assert!((place.left() - size.left()).abs() < 2.0);
    }
}
