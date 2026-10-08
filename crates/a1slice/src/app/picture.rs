//! The video picture and its play control.

use a1slice_core::crop::{self};
use a1slice_core::types::*;
use eframe::egui::{self, Color32};

use super::layout::{fit_picture, player_time_width, transport_slots};
use super::overlay::{self};
use super::paint::{
    paint_icon_hover, paint_pause_icon, paint_play_icon, paint_player_scrim, paint_player_time,
    player_seek,
};
use super::support::{active_cues, caption_on_picture, current_crop};
use super::theme::MUTED;
use super::A1App;

impl A1App {
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
        let shown_crop = framing.then(|| current_crop(&self.state, self.ratio));
        if let Some(crop) = shown_crop {
            overlay::paint_crop_frame(ui.painter(), image, crop);
        }

        let stage =
            egui::Rect::from_min_max(image.left_top(), egui::pos2(image.right(), bar.top()));
        let mut dragged = None;
        let mut scrolled = 0.0_f32;
        if stage.height() > 4.0 {
            let sense = if framing {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            };
            let stage_response = ui.interact(stage, ui.id().with("picture"), sense);
            if stage_response.clicked() {
                self.toggle_play();
            }
            if stage_response.hovered() {
                let icon = if framing && stage_response.dragged() {
                    egui::CursorIcon::Grabbing
                } else if framing {
                    egui::CursorIcon::Grab
                } else {
                    egui::CursorIcon::PointingHand
                };
                ui.ctx().set_cursor_icon(icon);
            }
            if framing && stage_response.dragged() {
                dragged = Some(stage_response.drag_delta());
            }
            if framing && stage_response.hovered() {
                scrolled = ui.input(|input| input.smooth_scroll_delta.y);
                if scrolled != 0.0 {
                    // The page sits in a scroll area. A wheel over the picture zooms the frame.
                    ui.input_mut(|input| {
                        input.smooth_scroll_delta = egui::Vec2::ZERO;
                        input.raw_scroll_delta = egui::Vec2::ZERO;
                    });
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
        if self.state.screen == Screen::Captions && self.caption_look == CaptionLook::Burn {
            let cues = active_cues(&self.state, self.caption_source);
            if let Some(text) = caption_on_picture(&cues, self.playhead_ms) {
                overlay::paint_caption(ui.painter(), image, text, &self.caption_style);
            }
        }

        if framing {
            let displayed = (image.width() as f64, image.height() as f64);
            let mut crop = current_crop(&self.state, self.ratio);
            let mut changed = false;
            if let Some(local) = shown_crop
                .map(|crop| overlay::crop_box(image, crop))
                .and_then(|frame| overlay::corner_drag(ui, frame, image))
            {
                crop = crop::zoom_from_corner(
                    crop,
                    local.x as f64,
                    local.y as f64,
                    displayed.0,
                    displayed.1,
                );
                changed = true;
            } else if let Some(delta) = dragged {
                crop = crop::pan_crop(
                    crop,
                    delta.x as f64,
                    delta.y as f64,
                    displayed.0,
                    displayed.1,
                );
                changed = true;
            }
            if scrolled != 0.0 {
                crop.zoom = crop::zoom_from_scroll(crop.zoom, scrolled as f64);
                crop = self.snap_crop_in(crop, displayed.0, displayed.1);
                changed = true;
            }
            if changed {
                self.store_crop(crop);
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
