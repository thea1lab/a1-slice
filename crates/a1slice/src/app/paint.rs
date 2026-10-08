//! Icons, the seek bar, and the sunset stripe.

use a1slice_core::clips::ClipViewWindow;
use a1slice_core::types::*;
use eframe::egui::{self, Color32, Stroke};

use super::layout::{ms_to_x, SeekHit};
use super::support::clock;
use super::theme::{CREAM_HEAD, INK, ORANGE};

pub(super) fn player_seek(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    image: egui::Rect,
    playhead_ms: i64,
    duration_ms: i64,
) -> Option<SeekHit> {
    let response = ui.interact(
        rect,
        ui.id().with("seek"),
        egui::Sense::CLICK | egui::Sense::DRAG,
    );
    let duration = duration_ms.max(1);
    let mut ms = playhead_ms.clamp(0, duration);
    let (span_left, span_right) = level_span(rect);
    if let Some(pos) = response
        .interact_pointer_pos()
        .filter(|_| response.dragged() || response.clicked() || response.drag_stopped())
    {
        ms = a1slice_core::preview::playback_ms_at(
            pos.x as f64,
            span_left as f64,
            (span_right - span_left) as f64,
            0.0,
            duration as f64,
        ) as i64;
    }
    let ratio = (ms as f32 / duration as f32).clamp(0.0, 1.0);
    let hot = response.hovered() || response.dragged();
    paint_level(ui.painter(), rect, ratio, hot, ORANGE);
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if let Some(pos) = response
        .hover_pos()
        .filter(|_| response.hovered() || response.dragged())
    {
        let tip_ms = if response.dragged() {
            ms
        } else {
            a1slice_core::preview::playback_ms_at(
                pos.x as f64,
                span_left as f64,
                (span_right - span_left) as f64,
                0.0,
                duration as f64,
            ) as i64
        };
        paint_seek_tip(ui.painter(), pos.x, rect.top(), &clock(tip_ms), image);
    }
    if response.dragged() {
        Some(SeekHit { ms, dragging: true })
    } else if response.clicked() || response.drag_stopped() {
        Some(SeekHit {
            ms,
            dragging: false,
        })
    } else {
        None
    }
}

pub(super) fn level_span(rect: egui::Rect) -> (f32, f32) {
    let left = rect.left() + 7.0;
    let right = (rect.right() - 7.0).max(left + 4.0);
    (left, right)
}

pub(super) fn paint_level(
    painter: &egui::Painter,
    rect: egui::Rect,
    ratio: f32,
    hot: bool,
    fill: Color32,
) {
    let ratio = ratio.clamp(0.0, 1.0);
    let track_h = if hot { 6.0 } else { 4.0 };
    let (left, right) = level_span(rect);
    let track = egui::Rect::from_min_max(
        egui::pos2(left, rect.center().y - track_h * 0.5),
        egui::pos2(right, rect.center().y + track_h * 0.5),
    );
    painter.rect_filled(track, track_h * 0.5, Color32::from_white_alpha(80));
    if ratio > 0.0 {
        let mut played = track;
        played.set_right(track.left() + track.width() * ratio);
        painter.rect_filled(played, track_h * 0.5, fill);
    }
    let knob = egui::pos2(track.left() + track.width() * ratio, track.center().y);
    painter.circle_filled(knob, if hot { 7.0 } else { 5.5 }, CREAM_HEAD);
}

pub(super) fn paint_player_time(
    ui: &egui::Ui,
    rect: egui::Rect,
    playhead_ms: i64,
    duration_ms: i64,
) {
    let font = egui::FontId::proportional(13.0);
    let current = ui
        .painter()
        .layout_no_wrap(clock(playhead_ms), font.clone(), CREAM_HEAD);
    let rest = ui.painter().layout_no_wrap(
        format!(" / {}", clock(duration_ms)),
        font,
        Color32::from_rgba_unmultiplied(255, 248, 224, 200),
    );
    let width = current.size().x + rest.size().x;
    let x = rect.right() - width;
    let y = rect.center().y - current.size().y * 0.5;
    ui.painter().galley(egui::pos2(x, y), current, CREAM_HEAD);
    ui.painter()
        .galley(egui::pos2(x + width - rest.size().x, y), rest, CREAM_HEAD);
}

pub(super) fn paint_clock(ui: &egui::Ui, rect: egui::Rect, playhead_ms: i64) {
    let galley = ui.painter().layout_no_wrap(
        clock(playhead_ms),
        egui::FontId::proportional(13.0),
        CREAM_HEAD,
    );
    let y = rect.center().y - galley.size().y * 0.5;
    ui.painter()
        .galley(egui::pos2(rect.left(), y), galley, CREAM_HEAD);
}

pub(super) fn paint_edge_label(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(13.0),
        CREAM_HEAD,
    );
    let y = rect.center().y - galley.size().y * 0.5;
    ui.painter()
        .galley(egui::pos2(rect.left(), y), galley, CREAM_HEAD);
}

pub(super) fn paint_clip_track(
    painter: &egui::Painter,
    rect: egui::Rect,
    view: ClipViewWindow,
    start_ms: i64,
    end_ms: i64,
    play_ms: i64,
    hot: bool,
) {
    let (left, right) = level_span(rect);
    let mid = rect.center().y;
    let track_h = if hot { 6.0 } else { 4.0 };
    let track = egui::Rect::from_min_max(
        egui::pos2(left, mid - track_h * 0.5),
        egui::pos2(right, mid + track_h * 0.5),
    );
    painter.rect_filled(track, track_h * 0.5, Color32::from_white_alpha(80));
    let x0 = ms_to_x(start_ms, view, left, right);
    let x1 = ms_to_x(end_ms, view, left, right);
    let fill = egui::Rect::from_min_max(
        egui::pos2(x0.min(x1), track.top()),
        egui::pos2(x0.max(x1).max(x0.min(x1) + 2.0).min(right), track.bottom()),
    );
    painter.rect_filled(fill, track_h * 0.5, ORANGE);
    let play_x = ms_to_x(play_ms, view, left, right);
    painter.line_segment(
        [
            egui::pos2(play_x, mid - 10.0),
            egui::pos2(play_x, mid + 10.0),
        ],
        Stroke::new(1.5_f32, CREAM_HEAD),
    );
    painter.circle_filled(
        egui::pos2(play_x, mid),
        if hot { 6.0 } else { 5.0 },
        CREAM_HEAD,
    );
    for x in [x0, x1] {
        let handle = egui::Rect::from_center_size(egui::pos2(x, mid), egui::vec2(4.0, 18.0));
        painter.rect_filled(handle, 1.5, ORANGE);
    }
}

pub(super) fn paint_chevron(painter: &egui::Painter, rect: egui::Rect, left: bool) {
    let center = rect.center();
    // A positive direction puts the tip on the left.
    let direction = if left { 1.0 } else { -1.0 };
    let reach = (rect.width().min(rect.height()) * 0.16).clamp(6.0, 14.0);
    let rise = reach * 1.15;
    let stroke = Stroke::new((reach * 0.22).clamp(1.8, 2.8), CREAM_HEAD);
    painter.line_segment(
        [
            center + egui::vec2(direction * reach, -rise),
            center + egui::vec2(-direction * reach * 0.75, 0.0),
        ],
        stroke,
    );
    painter.line_segment(
        [
            center + egui::vec2(-direction * reach * 0.75, 0.0),
            center + egui::vec2(direction * reach, rise),
        ],
        stroke,
    );
}

pub(super) fn paint_seek_tip(
    painter: &egui::Painter,
    anchor_x: f32,
    above: f32,
    label: &str,
    bounds: egui::Rect,
) {
    let galley = painter.layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(12.0),
        CREAM_HEAD,
    );
    let pad = egui::vec2(6.0, 3.0);
    let size = galley.size() + pad * 2.0;
    let x = (anchor_x - size.x * 0.5).clamp(
        bounds.left() + 4.0,
        (bounds.right() - size.x - 4.0).max(bounds.left() + 4.0),
    );
    let y = (above - size.y - 8.0).max(bounds.top() + 4.0);
    let tip = egui::Rect::from_min_size(egui::pos2(x, y), size);
    painter.rect_filled(tip, 4.0, Color32::from_black_alpha(220));
    painter.galley(tip.min + pad, galley, CREAM_HEAD);
}

pub(super) fn paint_player_scrim(painter: &egui::Painter, rect: egui::Rect) {
    if rect.height() < 1.0 || rect.width() < 1.0 {
        return;
    }
    // Vertex colors interpolate, so the fade is continuous instead of a stack of bars.
    let rows = 16;
    let mut mesh = egui::Mesh::default();
    for i in 0..=rows {
        let t = i as f32 / rows as f32;
        let y = egui::lerp(rect.top()..=rect.bottom(), t);
        let color = Color32::from_black_alpha((t * t * 210.0).round() as u8);
        mesh.colored_vertex(egui::pos2(rect.left(), y), color);
        mesh.colored_vertex(egui::pos2(rect.right(), y), color);
    }
    for i in 0..rows {
        let left = (i * 2) as u32;
        let right = left + 1;
        let next_left = left + 2;
        let next_right = left + 3;
        mesh.add_triangle(left, right, next_right);
        mesh.add_triangle(left, next_right, next_left);
    }
    painter.add(egui::Shape::mesh(mesh));
}

pub(super) fn paint_icon_hover(painter: &egui::Painter, response: &egui::Response) {
    if response.hovered() || response.is_pointer_button_down_on() {
        let alpha = if response.is_pointer_button_down_on() {
            42
        } else {
            24
        };
        painter.circle_filled(
            response.rect.center(),
            16.0,
            Color32::from_white_alpha(alpha),
        );
    }
}

pub(super) fn paint_play_icon(painter: &egui::Painter, rect: egui::Rect) {
    let c = rect.center() + egui::vec2(1.0, 0.0);
    let points = vec![
        egui::pos2(c.x - 5.0, c.y - 7.0),
        egui::pos2(c.x - 5.0, c.y + 7.0),
        egui::pos2(c.x + 7.0, c.y),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        CREAM_HEAD,
        Stroke::NONE,
    ));
}

pub(super) fn paint_pause_icon(painter: &egui::Painter, rect: egui::Rect) {
    let c = rect.center();
    for dx in [-3.6_f32, 3.6] {
        let bar = egui::Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(3.2, 14.0));
        painter.rect_filled(bar, 1.0, CREAM_HEAD);
    }
}

pub(super) fn paint_speaker(painter: &egui::Painter, rect: egui::Rect, muted: bool, volume: f32) {
    let c = rect.center();
    let body = egui::Rect::from_center_size(c + egui::vec2(-5.0, 0.0), egui::vec2(3.4, 6.4));
    painter.rect_filled(body, 0.6, CREAM_HEAD);
    let cone = vec![
        egui::pos2(body.right() - 0.4, body.top() + 0.6),
        egui::pos2(c.x + 1.2, c.y - 6.4),
        egui::pos2(c.x + 1.2, c.y + 6.4),
        egui::pos2(body.right() - 0.4, body.bottom() - 0.6),
    ];
    painter.add(egui::Shape::convex_polygon(cone, CREAM_HEAD, Stroke::NONE));
    let stroke = Stroke::new(1.6_f32, CREAM_HEAD);
    if muted || volume <= 0.001 {
        let x = c + egui::vec2(6.2, 0.0);
        painter.line_segment(
            [x + egui::vec2(-3.1, -3.1), x + egui::vec2(3.1, 3.1)],
            stroke,
        );
        painter.line_segment(
            [x + egui::vec2(-3.1, 3.1), x + egui::vec2(3.1, -3.1)],
            stroke,
        );
    } else {
        paint_wave(painter, c + egui::vec2(3.4, 0.0), 4.0, stroke);
        if volume >= 0.45 {
            paint_wave(painter, c + egui::vec2(3.4, 0.0), 7.0, stroke);
        }
    }
}

fn paint_wave(painter: &egui::Painter, center: egui::Pos2, radius: f32, stroke: Stroke) {
    let pts: Vec<_> = (0..=10)
        .map(|i| {
            let t = -0.9 + 1.8 * (i as f32 / 10.0);
            center + egui::vec2(radius * t.cos(), radius * t.sin())
        })
        .collect();
    painter.line(pts, stroke);
}

pub(super) fn paint_sunset(painter: &egui::Painter, rect: egui::Rect) {
    let stops = [
        (0.0_f32, Color32::from_rgb(0xfa, 0x52, 0x0f)),
        (0.30, Color32::from_rgb(0xff, 0x81, 0x05)),
        (0.55, Color32::from_rgb(0xff, 0xb8, 0x3e)),
        (0.78, Color32::from_rgb(0xff, 0xd9, 0x00)),
        (1.0, Color32::from_rgb(0xff, 0xf8, 0xe0)),
    ];
    let slices = rect.width().ceil().max(1.0) as i32;
    for i in 0..slices {
        let t0 = i as f32 / slices as f32;
        let t1 = (i + 1) as f32 / slices as f32;
        let x0 = egui::lerp(rect.left()..=rect.right(), t0);
        let x1 = egui::lerp(rect.left()..=rect.right(), t1);
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x1, rect.bottom())),
            0.0,
            sunset_at(&stops, (t0 + t1) * 0.5),
        );
    }
}

fn sunset_at(stops: &[(f32, Color32)], t: f32) -> Color32 {
    let mut prev = stops[0];
    for stop in stops.iter().copied() {
        if t <= stop.0 {
            let span = (stop.0 - prev.0).max(0.0001);
            return lerp_color(prev.1, stop.1, (t - prev.0) / span);
        }
        prev = stop;
    }
    stops.last().map(|stop| stop.1).unwrap_or(ORANGE)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

pub(super) fn paint_tool_icon(painter: &egui::Painter, tile: egui::Rect, tool: ToolId) {
    let origin = tile.center() - egui::vec2(16.0, 16.0);
    let stroke = Stroke::new(1.8_f32, INK);
    let p = |x: f32, y: f32| origin + egui::vec2(x, y);
    match tool {
        ToolId::Transcribe => {
            painter.rect_stroke(
                egui::Rect::from_min_size(p(12.0, 3.0), egui::vec2(8.0, 14.0)),
                egui::CornerRadius::same(4),
                stroke,
                egui::StrokeKind::Inside,
            );
            let arc: Vec<_> = (0..=14)
                .map(|i| {
                    let t = std::f32::consts::PI - std::f32::consts::PI * i as f32 / 14.0;
                    p(16.0 + 7.5 * t.cos(), 14.5 + 7.5 * t.sin())
                })
                .collect();
            painter.line(arc, stroke);
            painter.line(vec![p(16.0, 22.0), p(16.0, 26.5)], stroke);
            painter.line(vec![p(11.5, 26.5), p(20.5, 26.5)], stroke);
        }
        ToolId::Find => {
            painter.line(
                vec![p(6.0, 9.5), p(9.2, 6.0), p(22.8, 6.0), p(26.0, 9.5)],
                stroke,
            );
            painter.rect_stroke(
                egui::Rect::from_min_size(p(6.0, 9.5), egui::vec2(20.0, 14.0)),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line(vec![p(12.0, 16.5), p(20.0, 16.5)], stroke);
        }
        ToolId::Reframe => {
            painter.rect_stroke(
                egui::Rect::from_min_size(p(4.0, 8.0), egui::vec2(18.0, 16.0)),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                egui::Rect::from_min_size(p(14.0, 5.0), egui::vec2(14.0, 18.0)),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        ToolId::Captions => {
            painter.rect_stroke(
                egui::Rect::from_min_size(p(4.0, 6.0), egui::vec2(24.0, 16.0)),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line(vec![p(8.0, 16.5), p(18.0, 16.5)], stroke);
            painter.line(vec![p(8.0, 20.0), p(24.0, 20.0)], stroke);
        }
    }
}
