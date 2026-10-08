//! Playhead, frames, keyboard, and the review clip selection.

use crate::backend::{self};
use a1slice_core::clips::{self, CLIP_VIEW_PAD_MS};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::{self, WizardAction};
use eframe::egui::{self};
use std::sync::mpsc::{self};

use super::layout::playhead_after_trim;
use super::support::start_audio;
use super::{A1App, ReviewTrim};

pub(super) struct FrameResult {
    at_ms: i64,
    image: Result<(u32, u32, Vec<u8>), String>,
}

impl A1App {
    pub(super) fn poll_frame(&mut self, ctx: &egui::Context) {
        let ready = self.frame_rx.as_ref().and_then(|rx| rx.try_recv().ok());
        if let Some(ready) = ready {
            self.frame_rx = None;
            self.shown_ms = ready.at_ms;
            if let Ok((w, h, rgba)) = ready.image {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                self.frame = Some(ctx.load_texture("frame", image, egui::TextureOptions::LINEAR));
                if let Some(path) = self.state.video_path.clone() {
                    self.frame_for = Some((path, ready.at_ms));
                }
            }
        }
        if self.frame_rx.is_some() {
            ctx.request_repaint();
            return;
        }
        let Some(path) = self.state.video_path.clone() else {
            return;
        };
        let gap = (self.playhead_ms - self.shown_ms).abs();
        if self.frame.is_some() && gap < 80 {
            return;
        }
        let at_ms = self.playhead_ms;
        let (tx, rx) = mpsc::channel();
        self.frame_rx = Some(rx);
        std::thread::spawn(move || {
            let image = backend::grab_frame(std::path::Path::new(&path), at_ms, 960);
            let _ = tx.send(FrameResult { at_ms, image });
        });
        ctx.request_repaint();
    }

    pub(super) fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.restart_audio();
    }

    pub(super) fn restart_audio(&mut self) {
        self.stop_audio();
        if self.playing {
            if let Some(path) = self.state.video_path.clone() {
                self.audio = start_audio(&path, self.playhead_ms, self.audible_volume());
            }
        }
    }

    pub(super) fn audible_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume.clamp(0.0, 1.0)
        }
    }

    pub(super) fn seek_playhead(&mut self, ms: i64) {
        let end = self.state.video_duration_ms.max(0);
        let cap = if self.playing && end > 0 {
            end - 1
        } else {
            end
        };
        self.playhead_ms = ms.clamp(0, cap);
        if self.state.screen == Screen::Review {
            if let Some(clip) = self.review_clip() {
                self.loop_at_clip = self.playhead_ms < clip.end_ms;
            }
        }
        if self.playing {
            self.restart_audio();
        }
    }

    pub(super) fn player_keys(&mut self, ctx: &egui::Context) {
        let review = self.state.screen == Screen::Review && !self.state.clips.is_empty();
        let (play, back, forward, to_start, to_end, drop) = ctx.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::K),
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
                input.consume_key(egui::Modifiers::NONE, egui::Key::Home),
                input.consume_key(egui::Modifiers::NONE, egui::Key::End),
                review && input.consume_key(egui::Modifiers::NONE, egui::Key::D),
            )
        });
        if play {
            self.toggle_play();
        }
        if review {
            if back {
                self.step_review(-1);
            }
            if forward {
                self.step_review(1);
            }
            if drop {
                self.toggle_review_clip();
            }
            if let Some(clip) = self.review_clip() {
                if to_start {
                    self.seek_playhead(clip.start_ms);
                }
                if to_end {
                    self.seek_playhead(clip.end_ms.saturating_sub(1));
                }
            }
            return;
        }
        if to_start {
            self.seek_playhead(0);
        } else if back {
            self.seek_playhead(self.playhead_ms - 5_000);
        }
        if to_end {
            self.seek_playhead(self.state.video_duration_ms);
        } else if forward {
            self.seek_playhead(self.playhead_ms + 5_000);
        }
    }

    pub(super) fn video_limit_ms(&self) -> f64 {
        let end = self.state.video_duration_ms.max(0) as f64;
        if end > 0.0 {
            end
        } else {
            1.0
        }
    }

    pub(super) fn review_clip(&self) -> Option<ClipSegmentWithStatus> {
        if self.state.screen != Screen::Review || self.state.clips.is_empty() {
            return None;
        }
        let index = self.review_index.min(self.state.clips.len() - 1);
        Some(self.state.clips[index].clone())
    }

    pub(super) fn focus_review_clip(&mut self) {
        if self.state.screen != Screen::Review || self.state.clips.is_empty() {
            return;
        }
        if self.review_index >= self.state.clips.len() {
            self.review_index = self.state.clips.len() - 1;
        }
        let clip = self.state.clips[self.review_index].clone();
        if self.review_for.as_deref() == Some(clip.id.as_str()) {
            return;
        }
        self.review_view = Some(clips::clip_view_window(
            clip.start_ms,
            clip.end_ms,
            self.video_limit_ms(),
            CLIP_VIEW_PAD_MS,
        ));
        self.review_for = Some(clip.id);
        self.loop_at_clip = true;
        self.trim = ReviewTrim::Idle;
        self.seek_playhead(clip.start_ms);
    }

    pub(super) fn step_review(&mut self, delta: isize) {
        let len = self.state.clips.len() as isize;
        if len == 0 {
            return;
        }
        let next = (self.review_index as isize + delta).clamp(0, len - 1) as usize;
        if next == self.review_index {
            return;
        }
        self.review_index = next;
        self.review_for = None;
        self.focus_review_clip();
    }

    pub(super) fn toggle_review_clip(&mut self) {
        let Some(clip) = self.review_clip() else {
            return;
        };
        let discarding = clip.approved;
        let len = self.state.clips.len();
        self.dispatch(WizardAction::ToggleClip(clip.id));
        self.persist_clips();
        if discarding && self.review_index + 1 < len {
            self.step_review(1);
        }
    }

    pub(super) fn persist_clips(&self) {
        let Some(path) = &self.state.video_path else {
            return;
        };
        let stored: Vec<_> = self
            .state
            .clips
            .iter()
            .map(|clip| clip.to_stored())
            .collect();
        let raw = if self.state.raw_response.is_empty() {
            None
        } else {
            Some(self.state.raw_response.as_str())
        };
        let _ = sidecars::write_clips(std::path::Path::new(path), &stored, raw);
    }

    pub(super) fn set_review_edges(&mut self, id: &str, start_ms: i64, end_ms: i64) {
        self.state = wizard::wizard_reduce(
            &self.state,
            WizardAction::UpdateClipTimes {
                id: id.to_string(),
                start_ms,
                end_ms,
            },
        );
    }

    pub(super) fn begin_review_trim(&mut self) {
        self.loop_at_clip = true;
        if self.playing {
            self.stop_audio();
        }
    }

    pub(super) fn finish_review_trim(&mut self) {
        let resume = match self.trim {
            ReviewTrim::Handle { resume_ms, .. } | ReviewTrim::Clock { resume_ms, .. } => resume_ms,
            ReviewTrim::Idle => self.playhead_ms,
        };
        let preview = self.playhead_ms;
        if let Some(clip) = self.review_clip() {
            if let Some(view) = self.review_view {
                self.review_view = Some(clips::expand_view_to_fit(
                    view,
                    clip.start_ms,
                    clip.end_ms,
                    self.video_limit_ms(),
                    CLIP_VIEW_PAD_MS,
                ));
            }
            self.playhead_ms =
                playhead_after_trim(resume, preview, clip.start_ms, clip.end_ms, self.playing);
        }
        self.trim = ReviewTrim::Idle;
        self.scrubbing = false;
        // The new end is the loop point. A drag must not leave playback running past it.
        self.loop_at_clip = true;
        self.persist_clips();
        if self.playing {
            self.restart_audio();
        }
    }

    pub(super) fn stop_playback(&mut self) {
        self.playing = false;
        self.stop_audio();
    }

    pub(super) fn stop_audio(&mut self) {
        if let Some(mut child) = self.audio.take() {
            let _ = child.kill();
        }
    }
}
