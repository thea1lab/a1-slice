//! Dragging the review seek bar, clocks, and trim handles.

use a1slice_core::clips::{self, ClipViewWindow};
use a1slice_core::types::*;
use eframe::egui::{self};

use super::layout::ms_to_x;
use super::paint::{level_span, paint_icon_hover, paint_level, paint_seek_tip, paint_speaker};
use super::support::clock;
use super::theme::{CREAM_HEAD, HOLD_ARM_SECS, HOLD_STEP_SECS};
use super::{A1App, ReviewTrim};

impl A1App {
    pub(super) fn review_seek(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        image: egui::Rect,
        clip: &ClipSegmentWithStatus,
        view: ClipViewWindow,
    ) {
        let response = ui.interact(
            rect,
            ui.id().with("seek"),
            egui::Sense::CLICK | egui::Sense::DRAG,
        );
        let (left, right) = level_span(rect);
        let mut ms = self.playhead_ms;
        if let Some(pos) = response
            .interact_pointer_pos()
            .filter(|_| response.dragged() || response.clicked() || response.drag_stopped())
        {
            ms = a1slice_core::preview::playback_ms_at(
                pos.x as f64,
                left as f64,
                (right - left) as f64,
                view.view_start as f64,
                view.view_end as f64,
            ) as i64;
        }
        let live = self.review_clip().unwrap_or_else(|| clip.clone());
        if response.hovered() || response.dragged() {
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
                    left as f64,
                    (right - left) as f64,
                    view.view_start as f64,
                    view.view_end as f64,
                ) as i64
            };
            paint_seek_tip(ui.painter(), pos.x, rect.top(), &clock(tip_ms), image);
        }
        if response.dragged() {
            self.playhead_ms = ms.clamp(0, self.state.video_duration_ms.max(0));
            self.loop_at_clip = self.playhead_ms < live.end_ms;
            self.scrubbing = true;
        } else if response.clicked() || response.drag_stopped() {
            self.scrubbing = false;
            self.seek_playhead(ms);
        }
    }

    pub(super) fn paint_review_handles(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        clip: &ClipSegmentWithStatus,
        view: ClipViewWindow,
    ) {
        let (left, right) = level_span(rect);
        for (edge, ms) in [("start", clip.start_ms), ("end", clip.end_ms)] {
            let x = ms_to_x(ms, view, left, right);
            let hit = egui::Rect::from_center_size(
                egui::pos2(x, rect.center().y),
                egui::vec2(18.0, rect.height()),
            );
            let response = ui.interact(hit, ui.id().with(("clip-edge", edge)), egui::Sense::DRAG);
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            let same =
                matches!(self.trim, ReviewTrim::Handle { edge: current, .. } if current == edge);
            if response.dragged() && !same {
                let resume_ms = self.playhead_ms;
                self.begin_review_trim();
                self.trim = ReviewTrim::Handle {
                    edge,
                    frozen_start: clip.start_ms,
                    frozen_end: clip.end_ms,
                    view,
                    outward_since: None,
                    steps: 0,
                    step_origin: None,
                    last_step_at: 0.0,
                    resume_ms,
                };
            }
            if response.dragged() || response.drag_stopped() {
                self.drag_review_handle(ui, &response, edge, left, right, &clip.id);
            }
            if response.drag_stopped()
                && matches!(self.trim, ReviewTrim::Handle { edge: current, .. } if current == edge)
            {
                self.finish_review_trim();
            }
            response.on_hover_text(if edge == "start" {
                "Drag the start. Hold at the edge of the bar to go further."
            } else {
                "Drag the end. Hold at the edge of the bar to go further."
            });
        }
    }

    pub(super) fn drag_review_handle(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        edge: &'static str,
        left: f32,
        right: f32,
        id: &str,
    ) {
        let (
            frozen_start,
            frozen_end,
            view,
            mut outward_since,
            mut steps,
            mut step_origin,
            mut last_step_at,
            resume_ms,
        ) = match self.trim {
            ReviewTrim::Handle {
                edge: current,
                frozen_start,
                frozen_end,
                view,
                outward_since,
                steps,
                step_origin,
                last_step_at,
                resume_ms,
            } if current == edge => (
                frozen_start,
                frozen_end,
                view,
                outward_since,
                steps,
                step_origin,
                last_step_at,
                resume_ms,
            ),
            _ => return,
        };
        let Some(pos) = response.interact_pointer_pos() else {
            return;
        };
        let now = ui.input(|i| i.time);
        let slop = if steps > 0 { 16.0 } else { 3.0 };
        let outward = if edge == "start" {
            pos.x <= left + slop
        } else {
            pos.x >= right - slop
        };
        let limit = self.video_limit_ms();
        let pointer_ms = a1slice_core::preview::playback_ms_at(
            pos.x as f64,
            left as f64,
            (right - left) as f64,
            view.view_start as f64,
            view.view_end as f64,
        );
        let (start, end) = if outward {
            if outward_since.is_none() {
                outward_since = Some(now);
            }
            let mut pair = clips::trim_edge(edge, pointer_ms, frozen_start, frozen_end, limit);
            let since = outward_since.unwrap_or(now);
            if steps == 0 && now - since >= HOLD_ARM_SECS {
                steps = 1;
                step_origin = Some(if edge == "start" { pair.0 } else { pair.1 });
                last_step_at = now;
            } else if steps > 0 && now - last_step_at >= HOLD_STEP_SECS {
                steps += 1;
                last_step_at = now;
            }
            if steps > 0 {
                let origin = step_origin.unwrap_or(if edge == "start" {
                    frozen_start
                } else {
                    frozen_end
                });
                let delta = (if edge == "start" { -1.0 } else { 1.0 }) * f64::from(steps) * 1000.0;
                pair = clips::nudge_edge(
                    if edge == "start" {
                        origin
                    } else {
                        frozen_start
                    },
                    if edge == "end" { origin } else { frozen_end },
                    edge,
                    delta,
                    limit,
                );
            }
            pair
        } else {
            outward_since = None;
            steps = 0;
            step_origin = None;
            clips::trim_edge(edge, pointer_ms, frozen_start, frozen_end, limit)
        };
        self.trim = ReviewTrim::Handle {
            edge,
            frozen_start,
            frozen_end,
            view,
            outward_since,
            steps,
            step_origin,
            last_step_at,
            resume_ms,
        };
        self.set_review_edges(id, start, end);
        let play = if edge == "start" { start } else { end };
        self.playhead_ms = play.clamp(0, self.state.video_duration_ms.max(0));
        self.loop_at_clip = true;
        self.scrubbing = true;
        ui.ctx().request_repaint();
    }

    pub(super) fn player_volume(
        &mut self,
        ui: &mut egui::Ui,
        speaker: egui::Rect,
        slider: egui::Rect,
    ) {
        let show_slider = slider.width() >= 8.0;
        let mut volume = self.volume;
        let mut muted = self.muted;
        let mut restart = false;
        if show_slider {
            let response = ui.interact(
                slider,
                ui.id().with("volume"),
                egui::Sense::CLICK | egui::Sense::DRAG,
            );
            if let Some(pos) = response
                .interact_pointer_pos()
                .filter(|_| response.dragged() || response.clicked() || response.drag_stopped())
            {
                let (left, right) = level_span(slider);
                volume = ((pos.x - left) / (right - left).max(1.0)).clamp(0.0, 1.0);
                muted = volume <= 0.001;
            }
            if response.dragged() {
                self.volume = volume;
                self.muted = muted;
            }
            if response.drag_stopped() {
                self.volume = volume;
                self.muted = muted;
                restart = true;
            } else if response.clicked() {
                let changed = (volume - self.volume).abs() > 0.001 || muted != self.muted;
                self.volume = volume;
                self.muted = muted;
                restart = changed;
            }
            let shown = if self.muted { 0.0 } else { self.volume };
            paint_level(
                ui.painter(),
                slider,
                shown,
                response.hovered() || response.dragged(),
                CREAM_HEAD,
            );
            if response.hovered() || response.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            volume = self.volume;
            muted = self.muted;
        }
        let icon = ui.interact(speaker, ui.id().with("mute"), egui::Sense::CLICK);
        if icon.clicked() {
            muted = !muted;
            if !muted && volume <= 0.001 {
                volume = 1.0;
            }
            self.volume = volume;
            self.muted = muted;
            restart = true;
        }
        paint_icon_hover(ui.painter(), &icon);
        paint_speaker(ui.painter(), icon.rect, self.muted, self.volume);
        if icon.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let tip = if self.muted || self.volume <= 0.001 {
            "Unmute"
        } else {
            "Mute"
        };
        icon.on_hover_text(tip);
        if restart && self.playing {
            self.restart_audio();
        }
    }
}
