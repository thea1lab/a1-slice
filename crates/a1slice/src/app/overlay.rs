//! The crop frame and the caption line drawn on the picture.

use a1slice_core::captions::{caption_frame, caption_ink};
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

/// Draw the cue the way the burn will. Position, size, wrap, and outline come
/// from the same 384×288 ASS frame ffmpeg uses.
pub(super) fn paint_caption(
    painter: &egui::Painter,
    image: egui::Rect,
    text: &str,
    style: &CaptionStyle,
) {
    let text = text.trim();
    if text.is_empty() || image.height() < 8.0 || image.width() < 8.0 {
        return;
    }
    let placed = caption_frame(style, image.width() as f64, image.height() as f64);
    let font_px = (placed.font_px as f32).max(8.0);
    let margin_x = placed.margin_x as f32;
    let wrap = (image.width() - margin_x * 2.0).max(32.0);
    let ink = caption_ink(style);
    let color = color_from_hex(&ink.hex).unwrap_or(Color32::WHITE);
    let outline = if ink.outline.contains("FFFFFF") {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap;
    job.halign = egui::Align::Center;
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: caption_font(style.font, font_px),
            line_height: Some((font_px * 1.2).round().max(font_px)),
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    let galley = painter.layout_job(job);
    // The box we want on the picture. A centered galley keeps `rect` centered
    // on x = 0, so the paint position is not the top-left of that box.
    let box_left = image.center().x - galley.rect.width() * 0.5;
    let box_top = match placed.position {
        CaptionPosition::Top => image.top() + placed.margin_v as f32,
        CaptionPosition::Middle => image.center().y - galley.rect.height() * 0.5,
        CaptionPosition::Bottom => image.bottom() - placed.margin_v as f32 - galley.rect.height(),
    };
    let lowest = (image.bottom() - galley.rect.height()).max(image.top());
    let origin =
        egui::pos2(box_left, box_top.clamp(image.top(), lowest)) - galley.rect.min.to_vec2();
    let stroke = (placed.outline_px as f32).max(1.0);
    let painter = painter.with_clip_rect(image);
    for shift in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        painter.galley_with_override_text_color(
            origin + egui::vec2(shift.0 * stroke, shift.1 * stroke),
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
        CaptionFont::Mono => {
            egui::FontId::new(size, egui::FontFamily::Name("Noto Sans Mono".into()))
        }
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
    fn caption_sits_on_the_burn_margin() {
        let image = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(384.0, 288.0));
        let style = CaptionStyle {
            color: CaptionColor::White,
            custom_color: None,
            position: CaptionPosition::Bottom,
            size: CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        };
        let shapes = paint(|painter| paint_caption(painter, image, "Hello there", &style));
        let text = white_caption(shapes);
        assert!(text.galley.job.text.contains("Hello"));
        let visual = caption_box(&text);
        assert!(
            (visual.bottom() - (image.bottom() - 90.0)).abs() < 1.5,
            "large bottom caption should end 90px above the picture, bottom {}",
            visual.bottom()
        );
        assert!(
            (visual.center().x - image.center().x).abs() < 1.5,
            "the line should be centered on the picture, mid {}",
            visual.center().x
        );
        assert!(visual.left() >= image.left() - 1.0);
        assert!(visual.right() <= image.right() + 1.0);
    }

    #[test]
    fn a_long_caption_stays_on_the_picture() {
        let image = egui::Rect::from_min_size(egui::pos2(80.0, 40.0), egui::vec2(640.0, 360.0));
        let style = CaptionStyle {
            color: CaptionColor::White,
            custom_color: None,
            position: CaptionPosition::Bottom,
            size: CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        };
        let text = white_caption(paint(|painter| {
            paint_caption(
                painter,
                image,
                "Tá, eu vou gravar um vídeo\nentão do que eu estou fazendo\npara emitir a nota.",
                &style,
            )
        }));
        let visual = caption_box(&text);
        assert!(
            (visual.center().x - image.center().x).abs() < 1.5,
            "the block should be centered, mid {}",
            visual.center().x
        );
        assert!(
            visual.left() >= image.left() - 1.0,
            "the words ran off the left of the picture, left {}",
            visual.left()
        );
        assert!(visual.right() <= image.right() + 1.0);
    }

    #[test]
    fn top_caption_uses_the_top_margin() {
        let image = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(384.0, 288.0));
        let style = CaptionStyle {
            position: CaptionPosition::Top,
            size: CaptionSize::Small,
            ..CaptionStyle {
                color: CaptionColor::White,
                custom_color: None,
                position: CaptionPosition::Bottom,
                size: CaptionSize::Large,
                font_size: None,
                font: CaptionFont::Sans,
            }
        };
        let text = white_caption(paint(|painter| {
            paint_caption(painter, image, "Hello", &style)
        }));
        let visual = caption_box(&text);
        assert!(
            (visual.top() - (image.top() + 24.0)).abs() < 1.5,
            "small top caption should start 24px down, y {}",
            visual.top()
        );
        assert!((visual.center().x - image.center().x).abs() < 1.5);
    }

    fn caption_box(text: &egui::epaint::TextShape) -> egui::Rect {
        text.galley.rect.translate(text.pos.to_vec2())
    }

    fn white_caption(shapes: Vec<egui::Shape>) -> egui::epaint::TextShape {
        shapes
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Text(text) if text.override_text_color == Some(Color32::WHITE) => {
                    Some(text)
                }
                _ => None,
            })
            .expect("caption glyphs")
    }
}
