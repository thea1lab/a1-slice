//! The crop frame and the caption line drawn on the picture.

use a1slice_core::captions::{caption_ink, caption_metrics};
use a1slice_core::crop::{self};
use a1slice_core::types::*;
use eframe::egui::{self, Color32, Stroke};

use super::theme::{INK, ORANGE};

const FRAME_DIM: Color32 = Color32::from_black_alpha(158);

pub(super) fn crop_box(image: egui::Rect, crop: ClipCrop) -> egui::Rect {
    let placed = crop::crop_rect(crop, image.width() as f64, image.height() as f64);
    egui::Rect::from_min_size(
        image.min + egui::vec2(placed.x as f32, placed.y as f32),
        egui::vec2(placed.w as f32, placed.h as f32),
    )
}

pub(super) fn paint_crop_frame(painter: &egui::Painter, image: egui::Rect, crop: ClipCrop) {
    let frame = crop_box(image, crop);
    dim_outside(painter, image, frame);
    painter.rect_stroke(
        frame,
        0.0,
        Stroke::new(2.0_f32, ORANGE),
        egui::StrokeKind::Inside,
    );
    for handle in corner_handles(frame) {
        painter.rect_filled(handle, 2.0, ORANGE);
        painter.rect_stroke(
            handle,
            2.0,
            Stroke::new(2.0_f32, INK),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn corner_handles(frame: egui::Rect) -> [egui::Rect; 4] {
    let size = 14.0;
    // The handle hangs 6px outside the corner, the same offset as the main player.
    let at = |x: f32, y: f32| {
        egui::Rect::from_min_size(egui::pos2(x - 6.0, y - 6.0), egui::vec2(size, size))
    };
    [
        at(frame.left(), frame.top()),
        at(frame.right(), frame.top()),
        at(frame.left(), frame.bottom()),
        at(frame.right(), frame.bottom()),
    ]
}

/// A corner drag, in picture-local points. `None` when the pointer is not on a handle.
pub(super) fn corner_drag(
    ui: &mut egui::Ui,
    frame: egui::Rect,
    image: egui::Rect,
) -> Option<egui::Pos2> {
    for (index, handle) in corner_handles(frame).into_iter().enumerate() {
        let response = ui.interact(
            handle,
            ui.id().with(("crop-corner", index)),
            egui::Sense::drag(),
        );
        if response.hovered() || response.dragged() {
            let icon = if index == 0 || index == 3 {
                egui::CursorIcon::ResizeNwSe
            } else {
                egui::CursorIcon::ResizeNeSw
            };
            ui.ctx().set_cursor_icon(icon);
        }
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| response.dragged())
        {
            return Some(pos - image.min.to_vec2());
        }
    }
    None
}

pub(super) fn paint_caption(
    painter: &egui::Painter,
    image: egui::Rect,
    floor: f32,
    text: &str,
    style: &CaptionStyle,
) {
    let text = text.trim();
    if text.is_empty() || image.height() < 8.0 || image.width() < 8.0 {
        return;
    }
    let metrics = caption_metrics(style);
    let font_px = (metrics.font_size as f32 / 288.0) * image.height();
    let margin = (metrics.margin as f32 / 288.0) * image.height();
    let ink = caption_ink(style);
    let color = color_from_hex(&ink.hex).unwrap_or(Color32::WHITE);
    let outline = if ink.outline.contains("FFFFFF") {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = (image.width() * 0.86).max(32.0);
    job.halign = egui::Align::Center;
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: caption_font(style.font, font_px.max(8.0)),
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    let x = image.center().x - galley.size().x * 0.5;
    let mut y = match style.position {
        CaptionPosition::Top => image.top() + margin,
        CaptionPosition::Middle => image.center().y - galley.size().y * 0.5,
        CaptionPosition::Bottom => image.bottom() - margin - galley.size().y,
    };
    let lowest = floor - 6.0;
    if y + galley.size().y > lowest {
        y = lowest - galley.size().y;
    }
    y = y.max(image.top() + 4.0);
    let origin = egui::pos2(x, y);
    for shift in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        painter.galley_with_override_text_color(
            origin + egui::vec2(shift.0, shift.1),
            galley.clone(),
            outline,
        );
    }
    painter.galley_with_override_text_color(origin, galley, color);
}

fn dim_outside(painter: &egui::Painter, image: egui::Rect, hole: egui::Rect) {
    let hole = image.intersect(hole);
    if hole.width() < 1.0 || hole.height() < 1.0 {
        painter.rect_filled(image, 0.0, FRAME_DIM);
        return;
    }
    let bands = [
        egui::Rect::from_min_max(image.left_top(), egui::pos2(image.right(), hole.top())),
        egui::Rect::from_min_max(
            egui::pos2(image.left(), hole.bottom()),
            image.right_bottom(),
        ),
        egui::Rect::from_min_max(egui::pos2(image.left(), hole.top()), hole.left_bottom()),
        egui::Rect::from_min_max(hole.right_top(), egui::pos2(image.right(), hole.bottom())),
    ];
    for band in bands {
        if band.width() > 0.5 && band.height() > 0.5 {
            painter.rect_filled(band, 0.0, FRAME_DIM);
        }
    }
}

fn caption_font(font: CaptionFont, size: f32) -> egui::FontId {
    match font {
        CaptionFont::Serif => egui::FontId::new(size, egui::FontFamily::Name("Noto Serif".into())),
        CaptionFont::Mono => egui::FontId::monospace(size),
        CaptionFont::Sans => egui::FontId::proportional(size),
    }
}

fn color_from_hex(hex: &str) -> Option<Color32> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    Some(Color32::from_rgb(
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{crop_box, paint_caption, paint_crop_frame, FRAME_DIM, ORANGE};
    use a1slice_core::types::*;
    use eframe::egui::Color32;

    fn paint(mut draw: impl FnMut(&egui::Painter)) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw(ui.painter());
                });
            },
        );
        let mut shapes = Vec::new();
        for clipped in output.shapes {
            flatten(clipped.shape, &mut shapes);
        }
        shapes
    }

    fn flatten(shape: egui::Shape, out: &mut Vec<egui::Shape>) {
        if let egui::Shape::Vec(children) = shape {
            for child in children {
                flatten(child, out);
            }
        } else {
            out.push(shape);
        }
    }

    fn landscape() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(800.0, 450.0))
    }

    #[test]
    fn square_frame_is_drawn_inside_a_landscape_picture() {
        let image = landscape();
        let crop = ClipCrop {
            ratio: CropRatio::Square,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        };
        let shapes = paint(|painter| paint_crop_frame(painter, image, crop));
        let frame = shapes.iter().find_map(|shape| match shape {
            egui::Shape::Rect(rect) if rect.stroke.color == ORANGE && rect.stroke.width >= 2.0 => {
                Some(rect.rect)
            }
            _ => None,
        });
        let frame = frame.expect("orange crop frame");
        assert!(
            (frame.width() - image.height()).abs() < 1.0,
            "square width should match the picture height, got {}",
            frame.width()
        );
        assert!((frame.center().x - image.center().x).abs() < 1.0);
        assert!(
            frame.width() < image.width() - 40.0,
            "the sides stay outside the square"
        );
        let dimmed = shapes.iter().any(|shape| match shape {
            egui::Shape::Rect(rect) => rect.fill == FRAME_DIM && rect.rect.width() > 40.0,
            _ => false,
        });
        assert!(dimmed, "the picture outside the frame should be dimmed");
        assert_eq!(
            shapes
                .iter()
                .filter(|shape| matches!(shape, egui::Shape::Rect(rect) if rect.fill == ORANGE))
                .count(),
            4,
            "four corner handles"
        );
    }

    #[test]
    fn story_frame_is_a_tall_column() {
        let image = landscape();
        let crop = ClipCrop {
            ratio: CropRatio::R9x16,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        };
        let frame = crop_box(image, crop);
        assert!(frame.height() > image.height() - 1.0);
        assert!((frame.width() - image.height() * 9.0 / 16.0).abs() < 1.0);
        assert!((frame.center().x - image.center().x).abs() < 1.0);
    }

    #[test]
    fn caption_sits_on_the_picture_above_the_controls() {
        let image = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(480.0, 288.0));
        let floor = image.bottom() - 56.0;
        let style = CaptionStyle {
            color: CaptionColor::White,
            custom_color: None,
            position: CaptionPosition::Bottom,
            size: CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        };
        let shapes = paint(|painter| paint_caption(painter, image, floor, "Hello there", &style));
        let text = shapes.into_iter().find_map(|shape| match shape {
            egui::Shape::Text(text) if text.override_text_color == Some(Color32::WHITE) => {
                Some(text)
            }
            _ => None,
        });
        let text = text.expect("caption glyphs");
        assert!(text.galley.job.text.contains("Hello"));
        let bottom = text.pos.y + text.galley.size().y;
        assert!(
            bottom <= floor,
            "caption should sit above the control bar, bottom {bottom} floor {floor}"
        );
        assert!(
            text.pos.y > image.top() + 40.0,
            "a bottom caption should not jump to the top"
        );
    }
}
