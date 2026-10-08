//! Reframe screen and the saved crop.

use a1slice_core::crop::{self, CROP_PRESETS};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::layout::{picture_room, viewport_height};
use super::paint::{level_span, paint_level};
use super::support::current_crop;
use super::theme::{CREAM, CREAM_HEAD, HAIRLINE, HAIRLINE_HOVER, MUTED, ORANGE, SURFACE};
use super::widgets::primary_lg;
use super::A1App;

const CHIP_W: f32 = 96.0;
const CHIP_H: f32 = 84.0;
const CHIP_GAP: f32 = 8.0;
const SLIDER_COL: f32 = 300.0;
const PLACE_GAP: f32 = 36.0;

impl A1App {
    pub(super) fn reframe(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.state.video_path.is_none() {
            self.tool_intro(
                ui,
                ToolId::Reframe,
                "Reframe",
                "Crop the picture for a phone story, YouTube, a square, or 4:3. The frame is remembered for this video.",
                &["Pick the video.", "Choose a shape, then drag the frame until the subject sits inside it. Scroll to zoom.", "Export one new file."],
            );
            return;
        }
        let avail_w = ui.available_width();
        ui.set_max_width(avail_w);
        let side_by_side = controls_side_by_side(avail_w);
        let estimate = reframe_dock_height(side_by_side);
        let reserve = if self.reframe_dock_px > 1.0 {
            (self.reframe_dock_px + 8.0).max(estimate)
        } else {
            estimate
        };
        self.picture_limited(
            ui,
            ctx,
            avail_w,
            picture_room(viewport_height(ui), reserve, 200.0),
        );
        let dock_top = ui.cursor().min.y;
        ui.add_space(16.0);
        let cluster_w = cluster_width(avail_w);
        let side = ((avail_w - cluster_w) * 0.5).max(0.0);
        let ratio = self.ratio;
        let mut picked = None;
        let mut export = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(cluster_w);
                ui.label(RichText::new("Set the frame").size(18.0).color(CREAM_HEAD));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Choose a shape, then drag the frame on the video.")
                        .size(14.0)
                        .color(MUTED),
                );
                ui.add_space(16.0);
                if side_by_side {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.vertical(|ui| {
                            ui.set_width(shape_band_width());
                            picked = shape_choices(ui, ratio);
                        });
                        ui.add_space(PLACE_GAP);
                        ui.vertical(|ui| {
                            ui.set_width(SLIDER_COL);
                            export = self.place_column(ui, ratio);
                        });
                    });
                } else {
                    picked = shape_choices(ui, ratio);
                    ui.add_space(16.0);
                    export = self.place_column(ui, ratio);
                }
            });
        });
        if let Some(ratio) = picked {
            self.ratio = ratio;
            if ratio != CropRatio::Original {
                let crop = self.state.framing.get(&ratio).copied().unwrap_or(ClipCrop {
                    ratio,
                    cx: 0.5,
                    cy: 0.5,
                    zoom: 0.0,
                });
                self.store_crop(crop);
            }
        }
        if export {
            self.start_reframe();
        }
        ui.add_space(16.0);
        self.reframe_dock_px = (ui.cursor().min.y - dock_top).max(0.0);
    }

    fn place_column(&mut self, ui: &mut egui::Ui, ratio: CropRatio) -> bool {
        if ratio == CropRatio::Original {
            ui.label(
                RichText::new("The whole picture is kept.")
                    .size(14.0)
                    .color(MUTED),
            );
        } else {
            self.frame_sliders(ui);
        }
        ui.add_space(12.0);
        export_button(ui)
    }

    fn frame_sliders(&mut self, ui: &mut egui::Ui) {
        let crop = current_crop(&self.state, self.ratio);
        let mut next = crop;
        if let Some(value) = axis_slider(ui, "Horizontal", crop.cx as f32, 1.0, false) {
            next.cx = value as f64;
        }
        if let Some(value) = axis_slider(ui, "Vertical", crop.cy as f32, 1.0, false) {
            next.cy = value as f64;
        }
        if let Some(value) = axis_slider(ui, "Zoom", crop.zoom as f32, 2.0, true) {
            next.zoom = value as f64;
        }
        if next != crop {
            self.store_crop(self.snap_crop_in(next, 1.0, 1.0));
        }
    }

    pub(super) fn snap_crop_in(
        &self,
        crop: ClipCrop,
        fallback_w: f64,
        fallback_h: f64,
    ) -> ClipCrop {
        let (width, height) = if self.picture_px.0 >= 2 && self.picture_px.1 >= 2 {
            (self.picture_px.0 as f64, self.picture_px.1 as f64)
        } else if fallback_w >= 2.0 && fallback_h >= 2.0 {
            (fallback_w, fallback_h)
        } else {
            (1.0, 1.0)
        };
        crop::snap_crop_center(crop, width, height)
    }

    pub(super) fn store_crop(&mut self, crop: ClipCrop) {
        let mut framing = self.state.framing.clone();
        framing.insert(crop.ratio, crop);
        self.dispatch(WizardAction::SetFraming(framing.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_framing(std::path::Path::new(path), &framing);
        }
    }
}

fn shape_band_width() -> f32 {
    let count = CROP_PRESETS.len() as f32;
    count * CHIP_W + (count - 1.0) * CHIP_GAP + 4.0
}

fn controls_side_by_side(width: f32) -> bool {
    width >= shape_band_width() + PLACE_GAP + SLIDER_COL + 16.0
}

/// Width of the shape row plus the place column. It does not grow with the window.
fn cluster_width(avail: f32) -> f32 {
    if controls_side_by_side(avail) {
        shape_band_width() + PLACE_GAP + SLIDER_COL
    } else {
        avail
    }
}

/// Room under the picture so the shapes, sliders, and export stay above the stripe.
fn reframe_dock_height(side_by_side: bool) -> f32 {
    let heading = 22.0;
    let help = 22.0;
    let gaps = 16.0 + 4.0 + 16.0 + 16.0;
    let sliders = 32.0 * 3.0 + 8.0 * 2.0;
    let export = 48.0;
    let controls = if side_by_side {
        sliders + 12.0 + export
    } else {
        CHIP_H * 2.0 + CHIP_GAP + 16.0 + sliders + 12.0 + export
    };
    // The last term covers egui's item spacing, which the pieces above do not include.
    heading + help + gaps + controls + 40.0
}

fn export_button(ui: &mut egui::Ui) -> bool {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
        primary_lg(ui, "Export this frame").clicked()
    })
    .inner
}

fn shape_choices(ui: &mut egui::Ui, selected: CropRatio) -> Option<CropRatio> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(CHIP_GAP, CHIP_GAP);
        for preset in CROP_PRESETS {
            if shape_card(ui, preset.label, preset.ratio, selected == preset.ratio).clicked() {
                picked = Some(preset.ratio);
            }
        }
    });
    picked
}

fn shape_detail(ratio: CropRatio) -> &'static str {
    match ratio {
        CropRatio::Original => "Full",
        CropRatio::R9x16 => "9:16",
        CropRatio::R16x9 => "16:9",
        CropRatio::Square => "1:1",
        CropRatio::R4x3 => "4:3",
    }
}

fn shape_card(ui: &mut egui::Ui, label: &str, ratio: CropRatio, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(CHIP_W, CHIP_H), egui::Sense::click());
    let border = if selected {
        ORANGE
    } else if response.hovered() {
        HAIRLINE_HOVER
    } else {
        HAIRLINE
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
        Stroke::new(if selected { 1.5_f32 } else { 1.0_f32 }, border),
        egui::StrokeKind::Inside,
    );
    let mark = ratio_mark(ratio);
    let mark_rect =
        egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 22.0), mark);
    ui.painter().rect(
        mark_rect,
        egui::CornerRadius::same(2),
        Color32::TRANSPARENT,
        Stroke::new(1.5_f32, if selected { ORANGE } else { CREAM }),
        egui::StrokeKind::Inside,
    );
    let title = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.0),
        CREAM_HEAD,
    );
    let detail_color = if selected { CREAM } else { MUTED };
    let detail = ui.painter().layout_no_wrap(
        shape_detail(ratio).to_string(),
        egui::FontId::proportional(12.0),
        detail_color,
    );
    let title_h = title.size().y;
    let title_y = rect.top() + 42.0;
    ui.painter().galley(
        egui::pos2(rect.center().x - title.size().x * 0.5, title_y),
        title,
        CREAM_HEAD,
    );
    ui.painter().galley(
        egui::pos2(
            rect.center().x - detail.size().x * 0.5,
            title_y + title_h + 1.0,
        ),
        detail,
        detail_color,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

fn ratio_mark(ratio: CropRatio) -> egui::Vec2 {
    match ratio {
        CropRatio::Original => egui::vec2(26.0, 16.0),
        CropRatio::R16x9 => egui::vec2(28.0, 16.0),
        CropRatio::R4x3 => egui::vec2(24.0, 18.0),
        CropRatio::R9x16 => egui::vec2(15.0, 28.0),
        CropRatio::Square => egui::vec2(18.0, 18.0),
    }
}

fn unit_along(x: f32, left: f32, width: f32) -> f32 {
    if width <= 1.0 {
        return 0.0;
    }
    ((x - left) / width).clamp(0.0, 1.0)
}

fn axis_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: f32,
    max: f32,
    fit_at_zero: bool,
) -> Option<f32> {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, 32.0),
        egui::Sense::CLICK | egui::Sense::DRAG,
    );
    let label_w = 92.0_f32.min(width * 0.34);
    let value_w = 52.0;
    let gap = 10.0;
    let track_left = rect.left() + label_w + gap;
    let track_right = (rect.right() - value_w - gap).max(track_left + 24.0);
    let track_rect = egui::Rect::from_min_max(
        egui::pos2(track_left - 7.0, rect.top()),
        egui::pos2(track_right + 7.0, rect.bottom()),
    );
    let (span_left, span_right) = level_span(track_rect);
    let max = max.max(0.001);
    let mut shown = value.clamp(0.0, max);
    if let Some(pos) = response
        .interact_pointer_pos()
        .filter(|_| response.dragged() || response.clicked())
    {
        shown = unit_along(pos.x, span_left, span_right - span_left) * max;
    }
    let hot = response.hovered() || response.dragged();
    paint_level(ui.painter(), track_rect, shown / max, hot, ORANGE);
    let label_color = CREAM;
    let label_galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(15.0),
        label_color,
    );
    ui.painter().galley(
        egui::pos2(rect.left(), rect.center().y - label_galley.size().y * 0.5),
        label_galley,
        label_color,
    );
    let readout = if fit_at_zero && shown < 0.005 {
        "Fit".to_string()
    } else {
        format!("{:.0}%", shown * 100.0)
    };
    let value_galley =
        ui.painter()
            .layout_no_wrap(readout, egui::FontId::proportional(14.0), MUTED);
    ui.painter().galley(
        egui::pos2(
            rect.right() - value_galley.size().x,
            rect.center().y - value_galley.size().y * 0.5,
        ),
        value_galley,
        MUTED,
    );
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if (response.dragged() || response.clicked()) && (shown - value).abs() > 0.0005 {
        Some(shown)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cluster_width, controls_side_by_side, picture_room, reframe_dock_height, shape_detail,
        unit_along,
    };
    use a1slice_core::types::CropRatio;

    #[test]
    fn the_frame_controls_stay_together() {
        assert!(controls_side_by_side(1280.0));
        assert!(!controls_side_by_side(800.0));
        assert_eq!(cluster_width(1280.0), cluster_width(1600.0));
        assert!(cluster_width(1280.0) < 900.0);
        assert_eq!(cluster_width(800.0), 800.0);
        assert_eq!(shape_detail(CropRatio::R9x16), "9:16");
        assert_eq!(shape_detail(CropRatio::Original), "Full");
    }

    #[test]
    fn controls_stay_inside_the_opening_window() {
        let window_h = 960.0;
        let header = 48.0;
        let dock = reframe_dock_height(true);
        let picture = picture_room(window_h - header, dock, 200.0);
        assert!(picture + dock + header <= window_h + 0.5);
        assert!(dock > 280.0);
    }

    #[test]
    fn slider_follows_the_pointer() {
        assert!((unit_along(10.0, 0.0, 100.0) - 0.1).abs() < 0.001);
        assert_eq!(unit_along(-5.0, 0.0, 100.0), 0.0);
        assert_eq!(unit_along(150.0, 0.0, 100.0), 1.0);
        assert_eq!(unit_along(4.0, 0.0, 0.0), 0.0);
    }
}
