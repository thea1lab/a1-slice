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

pub(super) fn color_row(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    let current = caption_ink(style).hex;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        row_label(ui, "Color");
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
    });
    changed
}

pub(super) fn style_row(ui: &mut egui::Ui, style: &mut CaptionStyle, wide: bool) -> bool {
    let mut changed = false;
    if wide {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            changed |= place_chips(ui, style);
            ui.add_space(12.0);
            changed |= size_chips(ui, style);
            ui.add_space(12.0);
            changed |= font_chips(ui, style);
        });
    } else {
        changed |= chip_row(ui, |ui| place_chips(ui, style));
        ui.add_space(12.0);
        changed |= chip_row(ui, |ui| size_chips(ui, style));
        ui.add_space(12.0);
        changed |= chip_row(ui, |ui| font_chips(ui, style));
    }
    changed
}

/// A nested `horizontal` takes the whole cluster, so chips go straight into the row.
fn chip_row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> bool) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        changed = add(ui);
    });
    changed
}

fn place_chips(ui: &mut egui::Ui, style: &mut CaptionStyle) -> bool {
    let mut changed = false;
    row_label(ui, "Place");
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
    row_label(ui, "Size");
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
    row_label(ui, "Font");
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
    ui.label(RichText::new(text).size(14.0).color(MUTED));
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
    use super::style_row;
    use a1slice_core::types::*;

    #[test]
    fn wide_place_size_and_font_share_one_row() {
        let ctx = egui::Context::default();
        let mut style = CaptionStyle {
            color: CaptionColor::White,
            custom_color: None,
            position: CaptionPosition::Bottom,
            size: CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        };
        let mut height = 0.0_f32;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_max_width(1000.0);
                    let top = ui.cursor().min.y;
                    let _ = style_row(ui, &mut style, true);
                    height = ui.cursor().min.y - top;
                });
            },
        );
        assert!(
            height < 70.0,
            "place, size, and font should share one row, got {height}"
        );
    }
}
