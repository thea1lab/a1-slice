//! Review transport: play, clocks, and the trim handles' row.

use a1slice_core::clips::{self, CLIP_VIEW_PAD_MS, FINE_DRAG_PX};
use a1slice_core::types::*;
use eframe::egui::{self, Color32};

use super::layout::{edge_label_width, mono_clock_width, review_slots};
use super::paint::{paint_clip_track, paint_clock, paint_edge_label};
use super::support::clock;
use super::theme::CREAM_HEAD;
use super::{A1App, ReviewTrim};

impl A1App {
    pub(super) fn review_overlay(&mut self, ui: &mut egui::Ui, image: egui::Rect, bar: egui::Rect) {
        let Some(clip) = self.review_clip() else {
            return;
        };
        let stage =
            egui::Rect::from_min_max(image.left_top(), egui::pos2(image.right(), bar.top()));
        if !clip.approved && stage.height() > 0.0 {
            ui.painter()
                .rect_filled(stage, 0.0, Color32::from_black_alpha(110));
        }
        let wash_bottom = (image.top() + 64.0).min(bar.top());
        if wash_bottom > image.top() + 8.0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_max(image.left_top(), egui::pos2(image.right(), wash_bottom)),
                0.0,
                Color32::from_black_alpha(100),
            );
        }
        ui.painter().text(
            image.left_top() + egui::vec2(16.0, 10.0),
            egui::Align2::LEFT_TOP,
            &clip.title,
            egui::FontId::proportional(18.0),
            CREAM_HEAD,
        );
        ui.painter().text(
            image.left_top() + egui::vec2(16.0, 34.0),
            egui::Align2::LEFT_TOP,
            format!("{} – {}", clock(clip.start_ms), clock(clip.end_ms)),
            egui::FontId::proportional(14.0),
            Color32::from_white_alpha(200),
        );
        let outside = self.playhead_ms < clip.start_ms - 40 || self.playhead_ms > clip.end_ms + 40;
        if outside {
            ui.painter().text(
                egui::pos2(image.right() - 16.0, image.top() + 14.0),
                egui::Align2::RIGHT_TOP,
                "Outside the clip",
                egui::FontId::proportional(14.0),
                CREAM_HEAD,
            );
        }
    }

    pub(super) fn review_transport(
        &mut self,
        ui: &mut egui::Ui,
        bar: egui::Rect,
        image: egui::Rect,
        clip: &ClipSegmentWithStatus,
    ) {
        let duration = self.state.video_duration_ms.max(clip.end_ms);
        let start_w = edge_label_width(ui, "Start ", duration);
        let end_w = edge_label_width(ui, "End ", duration);
        let time_w = mono_clock_width(ui, duration).ceil();
        let slots = review_slots(bar, start_w, end_w, time_w);
        self.play_control(ui, slots.play);
        if slots.start_label.width() > 1.0 {
            self.review_clock(ui, slots.start_label, "start", clip);
        }
        let view = self.review_view.unwrap_or_else(|| {
            clips::clip_view_window(
                clip.start_ms,
                clip.end_ms,
                self.video_limit_ms(),
                CLIP_VIEW_PAD_MS,
            )
        });
        self.review_seek(ui, slots.seek, image, clip, view);
        if slots.end_label.width() > 1.0 {
            let clip = self.review_clip().unwrap_or_else(|| clip.clone());
            self.review_clock(ui, slots.end_label, "end", &clip);
        }
        let shown = self.review_clip().unwrap_or_else(|| clip.clone());
        paint_clock(ui, slots.time, self.playhead_ms);
        self.paint_review_handles(ui, slots.seek, &shown, view);
        let shown = self.review_clip().unwrap_or(shown);
        let hot = ui.input(|input| {
            input
                .pointer
                .hover_pos()
                .or(input.pointer.interact_pos())
                .is_some_and(|pos| slots.seek.contains(pos))
        });
        paint_clip_track(
            ui.painter(),
            slots.seek,
            view,
            shown.start_ms,
            shown.end_ms,
            self.playhead_ms,
            hot,
        );
        if self.scrubbing
            && ui.input(|i| i.pointer.any_released())
            && matches!(self.trim, ReviewTrim::Idle)
        {
            self.scrubbing = false;
            self.restart_audio();
        }
        self.player_volume(ui, slots.speaker, slots.slider);
    }

    pub(super) fn review_clock(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        edge: &'static str,
        clip: &ClipSegmentWithStatus,
    ) {
        let response = ui.interact(rect, ui.id().with(("clip-clock", edge)), egui::Sense::DRAG);
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        let same = matches!(self.trim, ReviewTrim::Clock { edge: current, .. } if current == edge);
        if response.dragged() && !same {
            let origin_x = response
                .interact_pointer_pos()
                .map(|pos| pos.x)
                .unwrap_or(rect.left());
            self.begin_review_trim();
            self.trim = ReviewTrim::Clock {
                edge,
                origin_x,
                origin_start: clip.start_ms,
                origin_end: clip.end_ms,
                applied: 0,
                resume_ms: self.playhead_ms,
            };
        }
        if response.dragged() || response.drag_stopped() {
            self.drag_review_clock(ui, &response, edge, &clip.id);
        }
        if response.drag_stopped()
            && matches!(self.trim, ReviewTrim::Clock { edge: current, .. } if current == edge)
        {
            self.finish_review_trim();
        }
        let live = self.review_clip().unwrap_or_else(|| clip.clone());
        let text = if edge == "start" {
            format!("Start {}", clock(live.start_ms))
        } else {
            format!("End {}", clock(live.end_ms))
        };
        paint_edge_label(ui, rect, &text);
        response.on_hover_text(if edge == "start" {
            "Drag to move the start by seconds"
        } else {
            "Drag to move the end by seconds"
        });
    }

    pub(super) fn drag_review_clock(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        edge: &'static str,
        id: &str,
    ) {
        let (origin_x, origin_start, origin_end, applied, resume_ms) = match self.trim {
            ReviewTrim::Clock {
                edge: current,
                origin_x,
                origin_start,
                origin_end,
                applied,
                resume_ms,
            } if current == edge => (origin_x, origin_start, origin_end, applied, resume_ms),
            _ => return,
        };
        let Some(pos) = response.interact_pointer_pos() else {
            return;
        };
        let seconds = clips::seconds_from_drag((pos.x - origin_x) as f64, FINE_DRAG_PX);
        if seconds == applied {
            self.scrubbing = true;
            return;
        }
        let (start, end) = clips::nudge_edge(
            origin_start,
            origin_end,
            edge,
            seconds as f64 * 1000.0,
            self.video_limit_ms(),
        );
        self.set_review_edges(id, start, end);
        let play = if edge == "start" { start } else { end };
        self.playhead_ms = play.clamp(0, self.state.video_duration_ms.max(0));
        self.loop_at_clip = true;
        self.scrubbing = true;
        self.trim = ReviewTrim::Clock {
            edge,
            origin_x,
            origin_start,
            origin_end,
            applied: seconds,
            resume_ms,
        };
        ui.ctx().request_repaint();
    }
}
