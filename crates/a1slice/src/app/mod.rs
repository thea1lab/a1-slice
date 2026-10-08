//! The A1 Slice window.
//!
//! `A1App` lives here. Each screen, the player, and the shared widgets
//! are sibling modules so they can use these fields.

mod captions;
mod export_screen;
mod find;
mod home;
mod jobs;
mod layout;
mod paint;
mod picture;
mod playback;
mod reframe;
mod review;
mod review_bar;
mod review_gestures;
mod support;
mod theme;
mod transcribe;
mod widgets;

use crate::backend::{self, Running};
use a1slice_core::clips::ClipViewWindow;
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::{self, WizardAction, WizardState};
use eframe::egui::{self, RichText};
use std::sync::mpsc::Receiver;
use std::time::Instant;

use paint::paint_sunset;
use playback::FrameResult;
use support::{default_style, load_recent, settings_of};
use theme::{apply_theme, CREAM_HEAD};
use widgets::text_hit;

enum ReviewTrim {
    Idle,
    Handle {
        edge: &'static str,
        frozen_start: i64,
        frozen_end: i64,
        view: ClipViewWindow,
        outward_since: Option<f64>,
        steps: i32,
        step_origin: Option<i64>,
        last_step_at: f64,
        /// Where playback was before the handle moved, so the loop can resume inside the clip.
        resume_ms: i64,
    },
    Clock {
        edge: &'static str,
        origin_x: f32,
        origin_start: i64,
        origin_end: i64,
        applied: i64,
        resume_ms: i64,
    },
}

pub struct A1App {
    state: WizardState,
    recent: Vec<(String, String)>,
    agents: Vec<(&'static str, &'static str)>,
    agent_id: Option<String>,
    job: Option<Running>,
    progress_rx: Option<Receiver<ProgressUpdate>>,
    frame: Option<egui::TextureHandle>,
    frame_for: Option<(String, i64)>,
    frame_rx: Option<Receiver<FrameResult>>,
    shown_ms: i64,
    playhead_ms: i64,
    playing: bool,
    scrubbing: bool,
    audio: Option<std::process::Child>,
    analyze_since: Option<Instant>,
    volume: f32,
    muted: bool,
    ratio: CropRatio,
    caption_style: CaptionStyle,
    caption_look: CaptionLook,
    caption_source: CaptionSource,
    fix_request: String,
    fix_log: String,
    pending_lines: Option<Vec<TranscriptSegment>>,
    status: String,
    review_index: usize,
    review_for: Option<String>,
    review_view: Option<ClipViewWindow>,
    loop_at_clip: bool,
    trim: ReviewTrim,
    /// Height of the review dock last frame, so the picture leaves room for it.
    review_dock_px: f32,
}

impl A1App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);
        let mut state = WizardState::default();
        let saved = sidecars::load_settings();
        state = wizard::wizard_reduce(
            &state,
            WizardAction::LoadSettings {
                provider: saved.provider,
                model: saved.model,
                api_key: saved.api_key,
                api_keys: saved.api_keys,
                user_hint: saved.user_hint,
                language: saved.language,
                entropy_thold: saved.entropy_thold,
                max_context: saved.max_context,
                beam_size: saved.beam_size,
                temperature_inc: saved.temperature_inc,
            },
        );
        let agents = backend::list_agents()
            .into_iter()
            .map(|(id, label, _)| (id, label))
            .collect::<Vec<_>>();
        let agent_id = agents.first().map(|(id, _)| (*id).to_string());
        Self {
            state,
            recent: load_recent(),
            agents,
            agent_id,
            job: None,
            progress_rx: None,
            frame: None,
            frame_for: None,
            frame_rx: None,
            shown_ms: -1,
            playhead_ms: 0,
            playing: false,
            scrubbing: false,
            audio: None,
            analyze_since: None,
            volume: 1.0,
            muted: false,
            ratio: CropRatio::R9x16,
            caption_style: default_style(),
            caption_look: CaptionLook::Burn,
            caption_source: CaptionSource::Transcript,
            fix_request: String::new(),
            fix_log: String::new(),
            pending_lines: None,
            status: String::new(),
            review_index: 0,
            review_for: None,
            review_view: None,
            loop_at_clip: true,
            trim: ReviewTrim::Idle,
            review_dock_px: 0.0,
        }
    }

    fn dispatch(&mut self, action: WizardAction) {
        self.state = wizard::wizard_reduce(&self.state, action);
        let _ = sidecars::save_settings(&settings_of(&self.state));
    }

    fn go_home(&mut self) {
        self.stop_playback();
        self.dispatch(WizardAction::GoHome);
    }

    fn enter(&mut self, tool: ToolId) {
        self.stop_playback();
        self.dispatch(WizardAction::EnterTool(tool));
        self.recent = load_recent();
    }

    fn draw(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.screen == Screen::Home {
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("A1 Slice").size(14.0).color(CREAM_HEAD));
                });
                ui.add_space(4.0);
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    if text_hit(ui, "A1 Slice", 15.0, false).clicked() {
                        self.go_home();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if text_hit(ui, "Home", 16.0, true).clicked() {
                            self.go_home();
                        }
                    });
                });
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.state.screen {
                    Screen::Home => self.home(ui),
                    Screen::Transcribe => self.transcribe(ui),
                    Screen::TranscribeDone => self.transcribe_done(ui),
                    Screen::Find => self.find(ui),
                    Screen::Review => self.review(ui, ctx),
                    Screen::Export => self.export_screen(ui),
                    Screen::Reframe => self.reframe(ui, ctx),
                    Screen::Captions => self.captions(ui, ctx),
                    Screen::FixWords => self.fix_words(ui),
                });
        });
    }
}

impl eframe::App for A1App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_job();
        self.tick_find_progress(ctx);
        if self.playing && !self.scrubbing {
            let dt = ctx.input(|i| i.stable_dt);
            self.playhead_ms += (dt * 1000.0) as i64;
            if let Some(clip) = self.review_clip() {
                let view_start = self
                    .review_view
                    .map(|view| view.view_start)
                    .unwrap_or(clip.start_ms);
                let view_end = self
                    .review_view
                    .map(|view| view.view_end)
                    .unwrap_or(clip.end_ms)
                    .max(view_start + 1);
                if self.loop_at_clip
                    && clip.end_ms > clip.start_ms
                    && self.playhead_ms >= clip.end_ms
                {
                    self.playhead_ms = clip.start_ms;
                    self.restart_audio();
                } else if !self.loop_at_clip && self.playhead_ms >= view_end {
                    self.playhead_ms = view_start;
                    self.restart_audio();
                }
            } else {
                let end = self.state.video_duration_ms.max(1);
                if self.playhead_ms >= end {
                    self.playhead_ms = 0;
                    self.stop_playback();
                }
            }
            ctx.request_repaint();
        }
        let show_picture = matches!(
            self.state.screen,
            Screen::Review | Screen::Reframe | Screen::Captions | Screen::FixWords
        ) && self.state.video_path.is_some();
        if show_picture {
            self.poll_frame(ctx);
            if !ctx.wants_keyboard_input() {
                self.player_keys(ctx);
            }
        }

        egui::TopBottomPanel::bottom("stripe")
            .exact_height(4.0)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                paint_sunset(ui.painter(), ui.max_rect());
            });
        self.draw(ctx);
    }
}
