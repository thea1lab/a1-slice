//! The video picture and its play control.

use a1slice_core::crop::{self};
use a1slice_core::types::*;
use eframe::egui::{self, Color32};

use super::layout::{fit_picture, player_time_width, transport_slots};
use super::paint::{
    paint_icon_hover, paint_pause_icon, paint_play_icon, paint_player_scrim, paint_player_time,
    player_seek,
};
use super::support::current_crop;
use super::theme::MUTED;
use super::A1App;

impl A1App {
    pub(super) fn picture(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.picture_limited(ui, ctx, ui.available_width(), 460.0);
    }

    pub(super) fn picture_limited(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        max_w: f32,
        max_h: f32,
    ) {
        let fitted = self
            .frame
            .as_ref()
            .map(|texture| texture.size_vec2())
            .map(|size| fit_picture(size.x, size.y, max_w, max_h))
            .unwrap_or_else(|| fit_picture(16.0, 9.0, max_w, max_h));
        let (row, _) = ui.allocate_exact_size(egui::vec2(max_w, fitted.y), egui::Sense::hover());
        let image = egui::Rect::from_center_size(row.center(), fitted);
        let Some(texture) = self.frame.clone() else {
            ui.painter().text(
                image.center(),
                egui::Align2::CENTER_CENTER,
                "Reading a frame…",
                egui::FontId::proportional(16.0),
                MUTED,
            );
            return;
        };
        ui.painter().image(
            texture.id(),
            image,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );

        let bar_h = 56.0_f32.min(image.height());
        let bar = egui::Rect::from_min_max(
            egui::pos2(image.left(), image.bottom() - bar_h),
            image.right_bottom(),
        );
        let scrim_top = (image.bottom() - 108.0).max(image.top());
        paint_player_scrim(
            ui.painter(),
            egui::Rect::from_min_max(egui::pos2(image.left(), scrim_top), image.right_bottom()),
        );

        let framing = self.state.screen == Screen::Reframe && self.ratio != CropRatio::Original;
        let stage =
            egui::Rect::from_min_max(image.left_top(), egui::pos2(image.right(), bar.top()));
        let mut dragged = None;
        let mut scrolled = 0.0_f32;
        if stage.height() > 4.0 {
            let sense = if framing {
                egui::Sense::CLICK | egui::Sense::DRAG
            } else {
                egui::Sense::CLICK
            };
            let stage_response = ui.interact(stage, ui.id().with("picture"), sense);
            if stage_response.clicked() {
                self.toggle_play();
            }
            if stage_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if framing {
                if stage_response.dragged() {
                    dragged = Some((stage_response.drag_delta(), image.width(), image.height()));
                }
                if stage_response.hovered() {
                    scrolled = ui.input(|i| i.smooth_scroll_delta.y);
                }
            }
        }

        let review = self.review_clip();
        if review.is_some() {
            self.review_overlay(ui, image, bar);
        }
        if let Some(clip) = review {
            self.review_transport(ui, bar, image, &clip);
        } else {
            let time_w = player_time_width(ui, self.state.video_duration_ms);
            let slots = transport_slots(bar, time_w);
            self.play_control(ui, slots.play);
            if let Some(hit) = player_seek(
                ui,
                slots.seek,
                image,
                self.playhead_ms,
                self.state.video_duration_ms,
            ) {
                self.playhead_ms = hit.ms;
                if hit.dragging {
                    self.scrubbing = true;
                } else {
                    self.scrubbing = false;
                    self.restart_audio();
                }
            } else if self.scrubbing && ui.input(|i| i.pointer.any_released()) {
                self.scrubbing = false;
                self.restart_audio();
            }
            paint_player_time(
                ui,
                slots.time,
                self.playhead_ms,
                self.state.video_duration_ms,
            );
            self.player_volume(ui, slots.speaker, slots.slider);
        }

        if framing {
            if let Some((delta, width, height)) = dragged {
                let mut crop = current_crop(&self.state, self.ratio);
                crop.cx = (crop.cx - delta.x as f64 / width as f64).clamp(0.0, 1.0);
                crop.cy = (crop.cy - delta.y as f64 / height as f64).clamp(0.0, 1.0);
                self.store_crop(crop::snap_crop_center(crop, 1.0, 1.0));
            }
            if scrolled != 0.0 {
                let mut crop = current_crop(&self.state, self.ratio);
                crop.zoom = (crop.zoom - scrolled as f64 / 400.0).clamp(0.0, 1.0);
                self.store_crop(crop::snap_crop_center(crop, 1.0, 1.0));
            }
        }
    }

    pub(super) fn play_control(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let play_tip = if self.playing { "Pause" } else { "Play" };
        let play = ui.interact(rect, ui.id().with("play"), egui::Sense::CLICK);
        paint_icon_hover(ui.painter(), &play);
        if self.playing {
            paint_pause_icon(ui.painter(), play.rect);
        } else {
            paint_play_icon(ui.painter(), play.rect);
        }
        if play.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if play.clicked() {
            self.toggle_play();
        }
        play.on_hover_text(play_tip);
    }
}
