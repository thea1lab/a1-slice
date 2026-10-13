//! Sheets, buttons, rows, and the home tiles.

use a1slice_core::types::*;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::paint::paint_tool_icon;
use super::theme::{BEIGE, CREAM, CREAM_HEAD, HAIRLINE, HAIRLINE_HOVER, MUTED, ORANGE, SURFACE};

pub(super) fn home_tile(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    title: &str,
    detail: &str,
    tool: ToolId,
    compact: bool,
) -> egui::Response {
    let response = ui.interact(rect, ui.id().with(title), egui::Sense::click());
    let border = if response.hovered() || response.has_focus() {
        HAIRLINE_HOVER
    } else {
        HAIRLINE
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(12),
        SURFACE,
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );
    let pad = if compact { 12.0 } else { 28.0 };
    let icon = if compact { 48.0 } else { 64.0 };
    let inner = rect.shrink2(egui::vec2(pad, pad));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.vertical_centered(|ui| {
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(icon, icon), egui::Sense::hover());
            ui.painter().rect(
                icon_rect,
                egui::CornerRadius::same(if compact { 8 } else { 12 }),
                CREAM_HEAD,
                Stroke::new(1.0_f32, BEIGE),
                egui::StrokeKind::Inside,
            );
            paint_tool_icon(ui.painter(), icon_rect, tool);
            ui.add_space(if compact { 8.0 } else { 16.0 });
            // Labels are not selectable. A selectable label sits on top of the
            // card, shows the text cursor, and keeps the click for itself.
            ui.add(tile_caption(
                title,
                if compact { 18.0 } else { 22.0 },
                CREAM_HEAD,
            ));
            ui.add_space(2.0);
            ui.add(tile_caption(
                detail,
                if compact { 13.0 } else { 15.0 },
                MUTED,
            ));
        });
    });
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn tile_caption(text: &str, size: f32, color: Color32) -> egui::Label {
    egui::Label::new(RichText::new(text).size(size).color(color)).selectable(false)
}

pub(super) fn with_sheet(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    let avail = ui.available_width();
    let sheet_w = avail.min(720.0).max(240.0);
    let side = ((avail - sheet_w) * 0.5).max(0.0);
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(side);
        ui.vertical(|ui| {
            ui.set_width(sheet_w);
            body(ui);
        });
    });
}

pub(super) fn primary_lg(ui: &mut egui::Ui, label: &str) -> egui::Response {
    primary_sized(ui, label, 48.0, 16.0)
}

pub(super) fn secondary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    painted_button(
        ui,
        label,
        14.0,
        CREAM,
        Color32::TRANSPARENT,
        Color32::from_white_alpha(16),
        Stroke::new(1.0_f32, Color32::from_rgb(0x4a, 0x4a, 0x4a)),
        40.0,
        20.0,
    )
}

pub(super) fn review_secondary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    painted_button(
        ui,
        label,
        14.0,
        CREAM,
        Color32::TRANSPARENT,
        Color32::from_white_alpha(16),
        Stroke::new(1.0_f32, Color32::from_rgb(0x4a, 0x4a, 0x4a)),
        48.0,
        20.0,
    )
}

pub(super) fn danger(ui: &mut egui::Ui, label: &str) -> egui::Response {
    painted_button(
        ui,
        label,
        14.0,
        CREAM,
        Color32::TRANSPARENT,
        Color32::from_rgb(0x3a, 0x20, 0x1c),
        Stroke::new(1.0_f32, Color32::from_rgb(0x7a, 0x3b, 0x32)),
        40.0,
        20.0,
    )
}

pub(super) fn choice(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let color = if selected { CREAM_HEAD } else { CREAM };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), egui::FontId::proportional(16.0), color);
    let size = egui::vec2((galley.size().x + 32.0).max(72.0), 44.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let border = if selected {
        ORANGE
    } else if response.hovered() {
        Color32::from_rgb(0x8a, 0x8a, 0x8a)
    } else {
        Color32::from_rgb(0x3a, 0x3a, 0x3a)
    };
    let fill = if selected {
        Color32::from_rgb(0x24, 0x1c, 0x18)
    } else {
        SURFACE
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(8),
        fill,
        Stroke::new(1.0_f32, border),
        egui::StrokeKind::Inside,
    );
    let pos = egui::pos2(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(pos, galley, color);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn primary_sized(ui: &mut egui::Ui, label: &str, height: f32, text: f32) -> egui::Response {
    painted_button(
        ui,
        label,
        text,
        Color32::WHITE,
        ORANGE,
        Color32::from_rgb(0xcc, 0x3a, 0x05),
        Stroke::NONE,
        height,
        22.0,
    )
}

fn painted_button(
    ui: &mut egui::Ui,
    label: &str,
    text_size: f32,
    text_color: Color32,
    fill: Color32,
    hover_fill: Color32,
    stroke: Stroke,
    height: f32,
    pad_x: f32,
) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(text_size),
        text_color,
    );
    let width = galley.size().x + pad_x * 2.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let fill = if response.hovered() { hover_fill } else { fill };
    let stroke = if response.hovered() && stroke.color == Color32::from_rgb(0x4a, 0x4a, 0x4a) {
        Stroke::new(1.0_f32, CREAM)
    } else {
        stroke
    };
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(8),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    let pos = egui::pos2(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(pos, galley, text_color);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

pub(super) fn text_hit(ui: &mut egui::Ui, label: &str, size: f32, chevron: bool) -> egui::Response {
    let font = egui::FontId::proportional(size);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, CREAM_HEAD);
    let extra = if chevron { 18.0 } else { 0.0 };
    let pad = egui::vec2(12.0, 8.0);
    let (rect, response) = ui.allocate_exact_size(
        galley.size() + pad + egui::vec2(extra, 0.0),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(8),
            Color32::from_white_alpha(16),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let text_pos = egui::pos2(
        rect.left() + 6.0 + extra,
        rect.center().y - galley.size().y * 0.5,
    );
    if chevron {
        let c = rect.left_center() + egui::vec2(10.0, 0.0);
        ui.painter().line(
            vec![
                c + egui::vec2(5.0, -5.0),
                c + egui::vec2(-1.0, 0.0),
                c + egui::vec2(5.0, 5.0),
            ],
            Stroke::new(1.6_f32, CREAM_HEAD),
        );
    }
    ui.painter().galley(text_pos, galley, CREAM_HEAD);
    response
}

/// Tool-screen bar. Home stays at the left, then the tool name, then the file.
pub(super) fn tool_nav(ui: &mut egui::Ui, title: &str, file: Option<&str>) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let home = text_hit(ui, "Home", 16.0, true);
        ui.add_space(8.0);
        ui.label(RichText::new(title).size(15.0).color(CREAM_HEAD));
        if let Some(file) = file.filter(|file| !file.is_empty()) {
            ui.add_space(8.0);
            ui.label(RichText::new(file).size(15.0).color(MUTED));
        }
        home
    })
    .inner
}

pub(super) fn recent_row(
    ui: &mut egui::Ui,
    name: &str,
    note: &str,
    first: bool,
    last: bool,
) -> egui::Response {
    let width = ui.available_width();
    let height = if note.is_empty() { 56.0 } else { 72.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    let radius = egui::CornerRadius {
        nw: if first { 12 } else { 0 },
        ne: if first { 12 } else { 0 },
        sw: if last { 12 } else { 0 },
        se: if last { 12 } else { 0 },
    };
    let fill = if response.hovered() {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    } else {
        SURFACE
    };
    ui.painter().rect(
        rect,
        radius,
        fill,
        Stroke::new(1.0_f32, HAIRLINE),
        egui::StrokeKind::Inside,
    );
    if !last {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            Stroke::new(1.0_f32, HAIRLINE),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let text = rect.shrink2(egui::vec2(16.0, 10.0));
    ui.painter().text(
        text.left_top(),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(16.0),
        CREAM_HEAD,
    );
    if !note.is_empty() {
        ui.painter().text(
            text.left_top() + egui::vec2(0.0, 24.0),
            egui::Align2::LEFT_TOP,
            note,
            egui::FontId::proportional(14.0),
            MUTED,
        );
    }
    response
}

pub(super) fn progress(ui: &mut egui::Ui, percent: f64) {
    let bar = egui::ProgressBar::new((percent as f32 / 100.0).clamp(0.0, 1.0))
        .text(format!("{percent:.0}%"));
    ui.add(bar);
}

pub(super) fn export_progress(ui: &mut egui::Ui, percent: f64) {
    let percent = percent.clamp(0.0, 100.0);
    let galley = ui.painter().layout_no_wrap(
        format!("{percent:.0}%"),
        egui::FontId::proportional(15.0),
        CREAM,
    );
    let label = galley.size();
    let gap = 16.0;
    let bar_h = 6.0;
    let width = ui.available_width();
    let row_h = label.y.max(bar_h);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_h), egui::Sense::hover());
    let bar_w = (width - gap - label.x).max(24.0);
    let track = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - bar_h * 0.5),
        egui::vec2(bar_w, bar_h),
    );
    ui.painter()
        .rect_filled(track, bar_h * 0.5, Color32::from_white_alpha(40));
    let fill_w = (bar_w * (percent as f32 / 100.0)).clamp(0.0, bar_w);
    if fill_w > 0.5 {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(track.min, egui::vec2(fill_w, bar_h)),
            bar_h * 0.5,
            ORANGE,
        );
    }
    ui.painter().galley(
        egui::pos2(rect.right() - label.x, rect.center().y - label.y * 0.5),
        galley,
        CREAM,
    );
}

#[cfg(test)]
mod tests {
    use super::{home_tile, tool_nav};
    use a1slice_core::types::ToolId;
    use eframe::egui::{self, CursorIcon, Pos2};

    fn probe(tile: egui::Rect, compact: bool, pos: Pos2) -> (bool, CursorIcon) {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 500.0));
        let mut input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(0.0),
            ..Default::default()
        };
        let mut clicked = false;
        let draw = |ctx: &egui::Context, clicked: &mut bool| {
            egui::CentralPanel::default().show(ctx, |ui| {
                *clicked = home_tile(
                    ui,
                    tile,
                    "Transcribe",
                    "Turn speech into a saved transcript.",
                    ToolId::Transcribe,
                    compact,
                )
                .clicked();
            });
        };
        let _ = ctx.run(input.clone(), |ctx| draw(ctx, &mut clicked));

        input.events.push(egui::Event::PointerMoved(pos));
        input.time = Some(0.05);
        let hover = ctx.run(input.clone(), |ctx| draw(ctx, &mut clicked));

        input.events.clear();
        input.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        });
        input.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        });
        input.time = Some(0.1);
        clicked = false;
        let _ = ctx.run(input, |ctx| draw(ctx, &mut clicked));
        (clicked, hover.platform_output.cursor_icon)
    }

    fn assert_whole_card_clicks(tile: egui::Rect, compact: bool) {
        let mut missed = Vec::new();
        let mut y = tile.top() + 6.0;
        while y < tile.bottom() - 6.0 {
            let pos = egui::pos2(tile.center().x, y);
            let (clicked, cursor) = probe(tile, compact, pos);
            if !clicked || cursor != CursorIcon::PointingHand {
                missed.push((y, clicked, format!("{cursor:?}")));
            }
            y += 12.0;
        }
        assert!(
            missed.is_empty(),
            "every point on the card, including the title, should click and show a hand. misses: {missed:?}"
        );
    }

    #[test]
    fn tool_nav_keeps_home_on_the_left() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 200.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut home = egui::Rect::NOTHING;
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                home = tool_nav(ui, "Transcribe", Some("interview.mp4")).rect;
            });
        });
        assert!(
            home.left() < 40.0,
            "Home starts at the left edge, got {}",
            home.left()
        );
        assert!(
            home.right() < screen.center().x,
            "Home stays on the left half, got {}",
            home.right()
        );
    }

    #[test]
    fn home_tile_clicks_through_the_title() {
        let wide = egui::Rect::from_min_size(egui::pos2(30.0, 24.0), egui::vec2(440.0, 210.0));
        assert_whole_card_clicks(wide, false);
        let narrow = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(360.0, 160.0));
        assert_whole_card_clicks(narrow, true);
    }
}
