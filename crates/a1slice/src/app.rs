//! The A1 Slice window. Same tools as the Electron app: transcribe, find, reframe, captions.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

use a1slice_core::clips::{self, ClipViewWindow, CLIP_VIEW_PAD_MS, FINE_DRAG_PX};
use a1slice_core::crop::{self, CROP_PRESETS};
use a1slice_core::sidecars::{self, with_clip_status};
use a1slice_core::types::*;
use a1slice_core::wizard::{self, ReturnTo, WizardAction, WizardState};
use eframe::egui::{self, Color32, RichText, Stroke};

use crate::backend::{self, JobDone, Running};

const CANVAS: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
const CREAM: Color32 = Color32::from_rgb(0xf4, 0xf1, 0xea);
const CREAM_HEAD: Color32 = Color32::from_rgb(0xff, 0xf8, 0xe0);
const MUTED: Color32 = Color32::from_rgb(0xa8, 0xa8, 0xa8);
const ORANGE: Color32 = Color32::from_rgb(0xfa, 0x52, 0x0f);
const SURFACE: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x1e);
const INK: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x1f);
const BEIGE: Color32 = Color32::from_rgb(0xe6, 0xd5, 0xa8);
const HAIRLINE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
const HAIRLINE_HOVER: Color32 = Color32::from_rgb(0xc7, 0xc7, 0xc7);
const HOLD_ARM_SECS: f64 = 0.280;
const HOLD_STEP_SECS: f64 = 0.340;

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

    fn open_video(&mut self, tool: ToolId, path: PathBuf) {
        let path_str = path.display().to_string();
        let _ = sidecars::write_remembered(&path_str);
        self.dispatch(WizardAction::OpenTool {
            tool,
            video_path: path_str.clone(),
        });
        let segments = sidecars::read_transcript(&path);
        let (clips, raw) = sidecars::read_clips(&path);
        let framing = sidecars::read_framing(&path);
        let captions = sidecars::read_captions(&path);
        if let Some(saved) = &captions {
            self.caption_style = saved.style.clone();
            self.caption_look = saved.look;
            self.caption_source = saved.source;
        }
        if let Some(saved) = framing.keys().copied().next() {
            self.ratio = saved;
        }
        let duration = backend::probe(&path).map(|(ms, _, _)| ms).unwrap_or(0);
        self.dispatch(WizardAction::ProjectLoaded {
            segments,
            clips: with_clip_status(&clips),
            raw_response: raw,
            framing,
            captions,
            duration_ms: duration,
        });
        self.playhead_ms = 0;
        self.review_index = 0;
        self.review_for = None;
        self.review_view = None;
        self.trim = ReviewTrim::Idle;
        self.frame = None;
        self.frame_for = None;
        self.frame_rx = None;
        self.shown_ms = -1;
        self.scrubbing = false;
        self.recent = load_recent();
    }

    fn choose_video(&mut self, tool: ToolId) {
        let file = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "avi", "webm"])
            .pick_file();
        if let Some(path) = file {
            self.open_video(tool, path);
        }
    }

    fn poll_job(&mut self) {
        let updates: Vec<ProgressUpdate> = self
            .progress_rx
            .as_ref()
            .map(|rx| {
                let mut batch = Vec::new();
                while let Ok(update) = rx.try_recv() {
                    batch.push(update);
                }
                batch
            })
            .unwrap_or_default();
        let screen = self.state.screen;
        for update in updates {
            match screen {
                Screen::Transcribe => self.dispatch(WizardAction::TranscribeProgress(update)),
                Screen::Find => self.dispatch(WizardAction::AnalyzeProgress(update)),
                Screen::Export | Screen::Reframe | Screen::Captions => {
                    self.dispatch(WizardAction::ExportProgress(update))
                }
                _ => {}
            }
        }
        let finished = self
            .job
            .as_ref()
            .is_some_and(|job| job.handle.is_finished());
        if !finished {
            return;
        }
        let job = self.job.take().unwrap();
        self.progress_rx = None;
        self.analyze_since = None;
        match job.handle.join() {
            Ok(Ok(JobDone::Transcript(segments))) => {
                if let Some(path) = self.state.video_path.clone() {
                    let _ = sidecars::write_transcript(std::path::Path::new(&path), &segments);
                }
                self.dispatch(WizardAction::TranscribeDone(segments));
            }
            Ok(Ok(JobDone::Clips { clips, raw })) => {
                let with = with_clip_status(&clips);
                if let Some(path) = self.state.video_path.clone() {
                    let _ = sidecars::write_clips(std::path::Path::new(&path), &clips, Some(&raw));
                }
                self.dispatch(WizardAction::AnalyzeDone {
                    clips: with,
                    raw_response: raw,
                });
                self.review_index = 0;
                self.review_for = None;
                self.review_view = None;
                self.trim = ReviewTrim::Idle;
            }
            Ok(Ok(JobDone::CaptionLines(lines))) => {
                self.pending_lines = Some(lines);
                self.fix_log
                    .push_str("\nThe edit is ready. Accept it to keep the new words.");
            }
            Ok(Ok(JobDone::Folder(path))) => {
                self.dispatch(WizardAction::ExportDone(path.display().to_string()));
            }
            Ok(Err(error)) if error == "Stopped." => {
                self.dispatch(WizardAction::TranscribeError(String::new()));
                self.state.transcribe_error = None;
                self.state.analyzing = false;
                self.state.analyze_error = None;
                self.state.analyze_message.clear();
            }
            Ok(Err(error)) => {
                if self.state.screen == Screen::Transcribe {
                    self.dispatch(WizardAction::TranscribeError(error));
                } else if self.state.analyzing {
                    self.dispatch(WizardAction::AnalyzeError(error));
                } else {
                    self.dispatch(WizardAction::ExportError(error));
                }
            }
            Err(_) => self.dispatch(WizardAction::ExportError(
                "The job stopped unexpectedly.".into(),
            )),
        }
    }

    fn start_transcribe(&mut self) {
        let Some(path) = self.state.video_path.clone() else {
            return;
        };
        let language = self.state.language;
        let entropy = self.state.entropy_thold;
        let max_context = self.state.max_context;
        let beam = self.state.beam_size;
        let temperature = self.state.temperature_inc;
        self.dispatch(WizardAction::StartTranscribe);
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let segments = backend::transcribe_video(
                std::path::Path::new(&path),
                language,
                entropy,
                max_context,
                beam,
                temperature,
                &cancel,
                &child,
                |update| {
                    let _ = tx.send(update);
                },
            )?;
            Ok(JobDone::Transcript(segments))
        }));
    }

    fn start_find(&mut self) {
        let Some(agent) = self.agent_id.clone() else {
            self.dispatch(WizardAction::AnalyzeError("Choose an agent.".into()));
            return;
        };
        let segments = self.state.segments.clone();
        let hint = self.state.user_hint.clone();
        self.dispatch(WizardAction::StartAnalyze);
        self.analyze_since = Some(Instant::now());
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let (clips, raw) = backend::find_clips(&segments, &agent, &hint, &cancel, &child)?;
            Ok(JobDone::Clips { clips, raw })
        }));
    }

    fn start_export_clips(&mut self) {
        let Some(path) = self.state.video_path.clone() else {
            return;
        };
        let clips: Vec<ClipSegment> = self
            .state
            .clips
            .iter()
            .filter(|clip| clip.approved)
            .map(|clip| clip.to_stored())
            .collect();
        if clips.is_empty() {
            self.status = "Keep at least one clip.".into();
            return;
        }
        let segments = self.state.segments.clone();
        self.dispatch(WizardAction::StartExport);
        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let dir = backend::export_kept_clips(
                std::path::Path::new(&path),
                &clips,
                &segments,
                &cancel,
                &child,
                |u| {
                    let _ = tx.send(u);
                },
            )?;
            Ok(JobDone::Folder(dir))
        }));
    }

    fn start_reframe(&mut self) {
        let Some(path) = self.state.video_path.clone() else {
            return;
        };
        let crop = current_crop(&self.state, self.ratio);
        self.dispatch(WizardAction::StartRender);
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let dir = backend::export_reframe(std::path::Path::new(&path), crop, &cancel, &child)?;
            Ok(JobDone::Folder(dir))
        }));
    }

    fn start_captions(&mut self) {
        let Some(path) = self.state.video_path.clone() else {
            return;
        };
        let cues = active_cues(&self.state, self.caption_source);
        let look = self.caption_look;
        let style = self.caption_style.clone();
        self.dispatch(WizardAction::StartRender);
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let out = backend::export_captions(
                std::path::Path::new(&path),
                &cues,
                look,
                &style,
                &cancel,
                &child,
            )?;
            Ok(JobDone::Folder(out))
        }));
    }

    fn start_fix(&mut self) {
        let Some(agent) = self.agent_id.clone() else {
            self.fix_log = "Choose an agent.".into();
            return;
        };
        let segments = active_cues(&self.state, self.caption_source);
        let request = self.fix_request.clone();
        self.fix_log = format!("Starting {agent}.");
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let lines = backend::fix_words(&segments, &agent, &request, &cancel, &child)?;
            Ok(JobDone::CaptionLines(lines))
        }));
    }

    fn cancel(&mut self) {
        if let Some(job) = &self.job {
            backend::cancel_job(job);
        }
    }

    fn tick_find_progress(&mut self, ctx: &egui::Context) {
        if !self.state.analyzing {
            return;
        }
        let since = *self.analyze_since.get_or_insert_with(Instant::now);
        let elapsed = since.elapsed().as_secs_f64();
        let percent = (6.0 + elapsed * 3.0).min(92.0);
        let message = if elapsed < 2.0 {
            "Reading the transcript…".to_string()
        } else {
            format!("Still working. {} seconds so far.", elapsed.round() as i64)
        };
        if (self.state.analyze_percent - percent).abs() > 0.3
            || self.state.analyze_message != message
        {
            self.dispatch(WizardAction::AnalyzeProgress(ProgressUpdate {
                stage: PipelineStage::Analyzing,
                message,
                percent,
            }));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }

    fn poll_frame(&mut self, ctx: &egui::Context) {
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

    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.restart_audio();
    }

    fn restart_audio(&mut self) {
        self.stop_audio();
        if self.playing {
            if let Some(path) = self.state.video_path.clone() {
                self.audio = start_audio(&path, self.playhead_ms, self.audible_volume());
            }
        }
    }

    fn audible_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume.clamp(0.0, 1.0)
        }
    }

    fn seek_playhead(&mut self, ms: i64) {
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

    fn player_keys(&mut self, ctx: &egui::Context) {
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

    fn video_limit_ms(&self) -> f64 {
        let end = self.state.video_duration_ms.max(0) as f64;
        if end > 0.0 {
            end
        } else {
            1.0
        }
    }

    fn review_clip(&self) -> Option<ClipSegmentWithStatus> {
        if self.state.screen != Screen::Review || self.state.clips.is_empty() {
            return None;
        }
        let index = self.review_index.min(self.state.clips.len() - 1);
        Some(self.state.clips[index].clone())
    }

    fn focus_review_clip(&mut self) {
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

    fn step_review(&mut self, delta: isize) {
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

    fn toggle_review_clip(&mut self) {
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

    fn persist_clips(&self) {
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

    fn set_review_edges(&mut self, id: &str, start_ms: i64, end_ms: i64) {
        self.state = wizard::wizard_reduce(
            &self.state,
            WizardAction::UpdateClipTimes {
                id: id.to_string(),
                start_ms,
                end_ms,
            },
        );
    }

    fn begin_review_trim(&mut self) {
        self.loop_at_clip = true;
        if self.playing {
            self.stop_audio();
        }
    }

    fn finish_review_trim(&mut self) {
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
            self.playhead_ms = playhead_after_trim(
                resume,
                preview,
                clip.start_ms,
                clip.end_ms,
                self.playing,
            );
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

    fn stop_playback(&mut self) {
        self.playing = false;
        self.stop_audio();
    }

    fn stop_audio(&mut self) {
        if let Some(mut child) = self.audio.take() {
            let _ = child.kill();
        }
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

impl A1App {
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

    fn home(&mut self, ui: &mut egui::Ui) {
        // Stay inside the visible panel. A centered line wider than the window
        // otherwise pulls the cards past the left clip edge.
        let bounds = ui.max_rect();
        let inset = 20.0;
        let width = (bounds.width() - inset * 2.0).max(160.0);
        let narrow = width < 720.0;
        let grid_w = width.min(920.0);
        let gap = 16.0;
        let cols: usize = if narrow { 1 } else { 2 };
        let tile_w = (grid_w - gap * (cols as f32 - 1.0)) / cols as f32;
        let tile_h = if narrow {
            let screen_h = ui.ctx().screen_rect().height();
            let chrome = 44.0;
            let header = 128.0;
            ((screen_h - chrome - header - gap * 3.0) / 4.0).clamp(136.0, 176.0)
        } else {
            210.0
        };
        let headline = if width < 760.0 {
            40.0
        } else if width < 1100.0 {
            48.0
        } else {
            56.0
        };
        let serif = egui::FontId::new(headline, egui::FontFamily::Name("Noto Serif".into()));

        ui.add_space(if narrow { 4.0 } else { 36.0 });
        ui.vertical_centered(|ui| {
            ui.set_max_width(width);
            ui.label(
                RichText::new("A1 SLICE")
                    .size(11.0)
                    .color(MUTED)
                    .strong()
                    .extra_letter_spacing(1.0),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("What do you want to do?")
                    .font(serif)
                    .color(CREAM_HEAD),
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new("Pick a tool. You’ll choose the video on the next screen.")
                    .size(16.0)
                    .color(MUTED),
            );
        });
        ui.add_space(if narrow { 16.0 } else { 40.0 });

        let tools = [
            (
                ToolId::Transcribe,
                "Transcribe",
                "Turn speech into a saved transcript.",
            ),
            (
                ToolId::Find,
                "Find best parts",
                "Keep the moments you want, then export them.",
            ),
            (
                ToolId::Reframe,
                "Reframe",
                "Fit the picture to Story, YouTube, or square.",
            ),
            (ToolId::Captions, "Captions", "Put the words on the video."),
        ];
        let rows = tools.len().div_ceil(cols);
        let grid_h = rows as f32 * tile_h + (rows.saturating_sub(1) as f32) * gap;
        let origin_y = ui.cursor().min.y;
        let left = bounds.left() + inset + ((width - grid_w) * 0.5).max(0.0);
        let _ = ui.allocate_exact_size(egui::vec2(bounds.width(), grid_h), egui::Sense::hover());
        let mut open = None;
        for (index, (id, title, detail)) in tools.into_iter().enumerate() {
            let col = index % cols;
            let row = index / cols;
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    left + col as f32 * (tile_w + gap),
                    origin_y + row as f32 * (tile_h + gap),
                ),
                egui::vec2(tile_w, tile_h),
            );
            if home_tile(ui, rect, title, detail, id, narrow).clicked() {
                open = Some(id);
            }
        }
        if let Some(id) = open {
            self.enter(id);
        }
        ui.add_space(80.0);
    }

    fn tool_intro(
        &mut self,
        ui: &mut egui::Ui,
        tool: ToolId,
        title: &str,
        lead: &str,
        steps: &[&str],
    ) {
        let avail = ui.available_width();
        let sheet_w = avail.min(720.0).max(240.0);
        let side = ((avail - sheet_w) * 0.5).max(0.0);
        let title_size = if sheet_w < 520.0 { 40.0 } else { 52.0 };
        let serif = egui::FontId::new(title_size, egui::FontFamily::Name("Noto Serif".into()));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(sheet_w);
                let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
                ui.painter().rect(
                    icon_rect,
                    egui::CornerRadius::same(12),
                    CREAM_HEAD,
                    Stroke::new(1.0_f32, BEIGE),
                    egui::StrokeKind::Inside,
                );
                paint_tool_icon(ui.painter(), icon_rect, tool);
                ui.add_space(28.0);
                ui.label(RichText::new(title).font(serif).color(CREAM_HEAD));
                ui.add_space(12.0);
                ui.label(RichText::new(lead).size(18.0).color(MUTED));
                ui.add_space(28.0);
                ui.label(RichText::new("What you do").size(18.0).color(CREAM_HEAD));
                ui.add_space(12.0);
                for (index, step) in steps.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new(format!("{:02}", index + 1)).size(15.0).color(ORANGE).strong());
                        ui.add_space(8.0);
                        ui.label(RichText::new(*step).size(18.0).color(CREAM));
                    });
                    ui.add_space(10.0);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    if primary_lg(ui, "Choose a video").clicked() {
                        self.choose_video(tool);
                    }
                    ui.label(RichText::new("MP4, MOV, MKV, AVI, or WebM").size(15.0).color(Color32::from_rgb(0x8a, 0x8a, 0x8a)));
                });
                if !self.recent.is_empty() {
                    ui.add_space(28.0);
                    ui.label(RichText::new("Recent videos").size(18.0).color(CREAM_HEAD));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Open a file you used before. To work on a different video, choose one above.").size(16.0).color(MUTED));
                    ui.add_space(10.0);
                    let picks = self.recent.clone();
                    let last = picks.len().saturating_sub(1);
                    for (index, (path, note)) in picks.into_iter().enumerate() {
                        if recent_row(ui, &file_name(&path), &note, index == 0, index == last).clicked() {
                            self.open_video(tool, PathBuf::from(path));
                        }
                    }
                }
            });
        });
    }

    fn transcribe(&mut self, ui: &mut egui::Ui) {
        if self.state.video_path.is_none() {
            self.tool_intro(
                ui,
                ToolId::Transcribe,
                "Transcribe",
                "Turn the speech in a video into a transcript. It stays on this computer, in a file next to the video.",
                &[
                    "Pick the video.",
                    "Tell us which language is spoken, or leave auto-detect on if you are not sure.",
                    "Start. The transcript is saved next to the video, and you can leave after that.",
                ],
            );
        } else if self.state.transcribe_stage == PipelineStage::Extracting
            || self.state.transcribe_stage == PipelineStage::Transcribing
            || self.state.transcribe_stage == PipelineStage::Downloading
        {
            ui.label(
                RichText::new("Transcribing")
                    .heading()
                    .size(36.0)
                    .color(CREAM),
            );
            ui.label(&self.state.transcribe_message);
            progress(ui, self.state.transcribe_percent);
            if ui.button("Cancel").clicked() {
                self.cancel();
            }
        } else if self.state.transcribe_stage == PipelineStage::Error {
            ui.label(
                RichText::new("Transcription failed")
                    .heading()
                    .color(ORANGE),
            );
            if let Some(error) = &self.state.transcribe_error {
                if !error.is_empty() {
                    ui.label(RichText::new(error).color(ORANGE));
                }
            }
            if primary(ui, "Try again").clicked() {
                self.start_transcribe();
            }
        } else {
            ui.label(
                RichText::new("Transcribe")
                    .heading()
                    .size(36.0)
                    .color(CREAM),
            );
            if let Some(path) = &self.state.video_path {
                ui.label(file_name(path));
            }
            ui.label("The speech stays on this computer.");
            ui.label(RichText::new("SPOKEN LANGUAGE").small().color(MUTED));
            ui.horizontal(|ui| {
                for (lang, label) in [
                    (VideoLanguage::Auto, "Auto-detect"),
                    (VideoLanguage::En, "English"),
                    (VideoLanguage::Pt, "Portuguese"),
                    (VideoLanguage::Es, "Spanish"),
                ] {
                    if ui
                        .selectable_label(self.state.language == lang, label)
                        .clicked()
                    {
                        self.dispatch(WizardAction::SetLanguage(lang));
                    }
                }
            });
            let saved = self.state.segments.len();
            if saved > 0 {
                ui.label(format!(
                    "This video already has a transcript ({saved} {}).",
                    if saved == 1 { "line" } else { "lines" }
                ));
                if primary(ui, "Use the saved transcript").clicked() {
                    self.dispatch(WizardAction::UseSavedTranscript);
                }
                if ui.button("Transcribe again").clicked() {
                    self.start_transcribe();
                }
            } else if primary(ui, "Start transcribing").clicked() {
                self.start_transcribe();
            }
            ui.collapsing("If the words come out wrong", |ui| {
            ui.label("Leave these as they are unless a transcript is missing words, full of junk, or stuck repeating itself.");
            ui.label("Drop uncertain words");
            let mut entropy = self.state.entropy_thold as f32;
            if ui.add(egui::Slider::new(&mut entropy, 0.0..=10.0).text("entropy")).changed() {
                self.dispatch(WizardAction::SetEntropyThold(entropy as f64));
            }
            let mut context = self.state.max_context as f32;
            if ui.add(egui::Slider::new(&mut context, 0.0..=128.0).text("max context")).changed() {
                self.dispatch(WizardAction::SetMaxContext(context as i64));
            }
            let mut beam = self.state.beam_size as f32;
            if ui.add(egui::Slider::new(&mut beam, 1.0..=8.0).text("beam size")).changed() {
                self.dispatch(WizardAction::SetBeamSize(beam as i64));
            }
            let mut temp = self.state.temperature_inc as f32;
            if ui.add(egui::Slider::new(&mut temp, 0.0..=1.0).text("temperature step")).changed() {
                self.dispatch(WizardAction::SetTemperatureInc(temp as f64));
            }
        });
        }
        ui.add_space(80.0);
    }

    fn transcribe_done(&mut self, ui: &mut egui::Ui) {
        let lines = self.state.segments.len();
        ui.label(
            RichText::new("Transcript saved")
                .heading()
                .size(36.0)
                .color(CREAM),
        );
        ui.label(format!(
            "{lines} {} saved in a text file in the same folder as the video.",
            if lines == 1 { "line is" } else { "lines are" }
        ));
        if let Some(path) = &self.state.video_path {
            let text = sidecars::transcript_text_path(std::path::Path::new(path));
            ui.label(RichText::new("THE FILE").small().color(MUTED));
            ui.label(text.display().to_string());
            if ui.button("Open transcription").clicked() {
                if let Err(error) = backend::open_path(&text) {
                    self.status = error;
                }
            }
        }
        for segment in &self.state.segments {
            ui.label(format!("{}  {}", clock(segment.start_ms), segment.text));
        }
        if !self.status.is_empty() {
            ui.label(RichText::new(&self.status).color(ORANGE));
        }
        if primary(ui, "Back to home").clicked() {
            self.go_home();
        }
    }

    fn find(&mut self, ui: &mut egui::Ui) {
        if self.state.video_path.is_none() {
            self.tool_intro(
                ui,
                ToolId::Find,
                "Find best parts",
                "Mark the moments worth keeping, fix where each one starts and ends, then export those clips.",
                &[
                    "Pick the video.",
                    "If it has no transcript yet, the words are written down first so they can be searched.",
                    "Keep or drop each part, nudge the start and end, then export.",
                ],
            );
            ui.add_space(80.0);
            return;
        }
        let serif = egui::FontId::new(52.0, egui::FontFamily::Name("Noto Serif".into()));
        with_sheet(ui, |ui| {
            if !self.state.project_ready {
                ui.label(RichText::new("Opening the video…").size(16.0).color(MUTED));
                return;
            }
            ui.label(
                RichText::new("Find best parts")
                    .font(serif.clone())
                    .color(CREAM_HEAD),
            );
            ui.add_space(12.0);
            if self.state.segments.is_empty() {
                ui.label(RichText::new("This video has no transcript yet. The words have to be written down before the best parts can be found.").size(18.0).color(MUTED));
                ui.add_space(20.0);
                if primary_lg(ui, "Transcribe this video").clicked() {
                    self.dispatch(WizardAction::PrepareTranscribe(ReturnTo::Find));
                }
                return;
            }
            ui.label(RichText::new("An agent reads the transcript and suggests clips, with a start and an end for each one. You can also mark a part yourself.").size(18.0).color(MUTED));
            ui.add_space(28.0);
            if !self.state.analyzing && !self.state.clips.is_empty() {
                let count = self.state.clips.len();
                egui::Frame::new()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0_f32, HAIRLINE))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(20.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!(
                            "A search is already saved for this video ({count} {}). Open it, or set up a new search below.",
                            if count == 1 { "clip" } else { "clips" }
                        )).size(16.0).color(CREAM));
                        ui.add_space(14.0);
                        if primary_lg(ui, "Open the saved search").clicked() {
                            self.dispatch(WizardAction::ShowScreen(Screen::Review));
                        }
                    });
                ui.add_space(28.0);
            }
            let lines = self.state.segments.len();
            egui::CollapsingHeader::new(
                RichText::new(format!(
                    "Transcript · {lines} {}",
                    if lines == 1 { "line" } else { "lines" }
                ))
                .size(18.0)
                .color(CREAM_HEAD),
            )
            .show(ui, |ui| {
                for segment in &self.state.segments {
                    ui.label(format!("{}  {}", clock(segment.start_ms), segment.text));
                }
            });
            ui.add_space(20.0);
            if !self.state.analyzing && secondary(ui, "Mark a part yourself").clicked() {
                self.dispatch(WizardAction::AddClip);
            }
            ui.add_space(28.0);
            ui.label(RichText::new("Which agent").size(18.0).color(CREAM_HEAD));
            ui.add_space(8.0);
            ui.label(RichText::new("The words are sent to this agent so it can suggest clips. The video stays on this computer.").size(16.0).color(MUTED));
            ui.add_space(12.0);
            if self.agents.is_empty() {
                ui.label(
                    RichText::new(
                        "This computer has no agent to run. You can still mark a part yourself.",
                    )
                    .size(16.0)
                    .color(MUTED),
                );
            } else {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    for (id, label) in self.agents.clone() {
                        let selected = self.agent_id.as_deref() == Some(id);
                        if choice(ui, label, selected).clicked() && !self.state.analyzing {
                            self.agent_id = Some(id.to_string());
                        }
                    }
                });
            }
            ui.add_space(16.0);
            ui.label(
                RichText::new("What should it look for?")
                    .size(16.0)
                    .color(MUTED),
            );
            ui.add_space(8.0);
            let mut hint = self.state.user_hint.clone();
            let hint_edit = ui.add(
                egui::TextEdit::multiline(&mut hint)
                    .desired_rows(3)
                    .hint_text("Two clips about the demo, or the part where the bug shows up.")
                    .desired_width(f32::INFINITY),
            );
            if hint_edit.changed() {
                self.dispatch(WizardAction::SetUserHint(hint));
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("Optional. Name a theme, a number of clips, or a moment to keep.")
                    .size(16.0)
                    .color(MUTED),
            );
            if self.state.analyzing {
                ui.add_space(16.0);
                progress(ui, self.state.analyze_percent);
                ui.label(
                    RichText::new(&self.state.analyze_message)
                        .size(15.0)
                        .color(MUTED),
                );
            }
            if let Some(error) = &self.state.analyze_error {
                if !error.is_empty() {
                    ui.add_space(16.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(0x3a, 0x20, 0x1c))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x7a, 0x3b, 0x32)))
                        .corner_radius(egui::CornerRadius::same(12))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Could not find clips")
                                    .size(16.0)
                                    .color(Color32::from_rgb(0xfc, 0xa5, 0xa5)),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(error)
                                    .size(14.0)
                                    .color(Color32::from_rgb(0xfc, 0xa5, 0xa5)),
                            );
                        });
                }
            }
            ui.add_space(16.0);
            if !self.state.analyzing
                && !self.state.clips.is_empty()
                && secondary(ui, "Back to the clips").clicked()
            {
                self.dispatch(WizardAction::ShowScreen(Screen::Review));
            }
            ui.add_space(12.0);
            if self.state.analyzing {
                if danger(ui, "Cancel").clicked() {
                    self.cancel();
                }
            } else {
                let label = if self.state.clips.is_empty() {
                    "Find clips"
                } else {
                    "Search again"
                };
                let enabled = self.agent_id.is_some();
                if primary_lg(ui, label).clicked() && enabled {
                    self.start_find();
                }
            }
        });
        ui.add_space(80.0);
    }

    fn review(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.focus_review_clip();
        let avail_w = ui.available_width();
        let viewport_h = ui.clip_rect().height();
        let avail_h = if viewport_h.is_finite() && viewport_h > 1.0 {
            viewport_h
        } else {
            640.0
        };
        let tex = self
            .frame
            .as_ref()
            .map(|texture| texture.size_vec2())
            .unwrap_or(egui::vec2(16.0, 9.0));
        // 8px is the gap the parent layout inserts between the picture and the dock.
        let dock = if self.review_dock_px > 1.0 {
            self.review_dock_px + 8.0
        } else {
            248.0
        };
        let frame = review_frame(avail_w, avail_h, tex.x, tex.y, dock);
        let row_w = frame.button * 2.0 + frame.gap * 2.0 + frame.picture.x;
        let side = ((avail_w - row_w) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side);
            self.review_nav(ui, frame.button, frame.picture.y, -1);
            ui.add_space(frame.gap);
            ui.vertical(|ui| {
                ui.set_width(frame.picture.x);
                self.picture_limited(ui, ctx, frame.picture.x, frame.picture.y);
            });
            ui.add_space(frame.gap);
            self.review_nav(ui, frame.button, frame.picture.y, 1);
        });
        let dock_row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side + frame.button + frame.gap);
            ui.vertical(|ui| {
                ui.set_width(frame.picture.x);
                self.review_column(ui);
            });
        });
        self.review_dock_px = dock_row.response.rect.height();
    }

    fn review_nav(&mut self, ui: &mut egui::Ui, button: f32, video_h: f32, delta: isize) {
        let (column, _) =
            ui.allocate_exact_size(egui::vec2(button, video_h.max(button)), egui::Sense::hover());
        let len = self.state.clips.len();
        if len == 0 {
            return;
        }
        let index = self.review_index.min(len - 1);
        let enabled = if delta < 0 {
            index > 0
        } else {
            index + 1 < len
        };
        if !enabled {
            return;
        }
        let hit = egui::Rect::from_center_size(column.center(), egui::vec2(button, button));
        let response = ui.interact(
            hit,
            ui.id().with(if delta < 0 { "prev-clip" } else { "next-clip" }),
            egui::Sense::CLICK,
        );
        let fill = if response.hovered() {
            Color32::from_rgb(0xcc, 0x3a, 0x05)
        } else {
            ORANGE
        };
        ui.painter().rect(
            hit,
            egui::CornerRadius::same(8),
            fill,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        paint_chevron(ui.painter(), hit, delta < 0);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            self.step_review(delta);
        }
        response.on_hover_text(if delta < 0 {
            "Previous clip"
        } else {
            "Next clip"
        });
    }

    fn review_column(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(16.0);
        let (hair, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(
            hair.x_range(),
            hair.center().y,
            Stroke::new(1.0_f32, HAIRLINE),
        );
        ui.add_space(16.0);
        let kept = self.state.clips.iter().filter(|clip| clip.approved).count();
        let total = self.state.clips.len();
        if let Some(clip) = self.review_clip() {
            let index = self.review_index.min(total.saturating_sub(1));
            self.review_identity(ui, &clip, index, total, kept);
        } else {
            ui.label(
                RichText::new("No clips to review yet.")
                    .size(16.0)
                    .color(MUTED),
            );
        }
        if !self.status.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(&self.status).color(ORANGE));
        }
        ui.add_space(20.0);
        self.review_actions(ui, kept);
        ui.add_space(16.0);
    }

    fn review_identity(
        &self,
        ui: &mut egui::Ui,
        clip: &ClipSegmentWithStatus,
        index: usize,
        total: usize,
        kept: usize,
    ) {
        let width = ui.available_width();
        let time_size = if width < 560.0 {
            36.0
        } else if width < 900.0 {
            44.0
        } else {
            52.0
        };
        let cream_soft = Color32::from_rgb(0xd9, 0xd3, 0xc5);
        let length = clip_length_words(clip.start_ms, clip.end_ms);
        let mut length_size_px = time_size;
        let mut length_galley = ui.painter().layout_no_wrap(
            length.clone(),
            egui::FontId::new(
                length_size_px,
                egui::FontFamily::Name("Noto Serif".into()),
            ),
            CREAM_HEAD,
        );
        while length_size_px > 32.0 && length_galley.size().x > width {
            length_size_px -= 4.0;
            length_galley = ui.painter().layout_no_wrap(
                length.clone(),
                egui::FontId::new(
                    length_size_px,
                    egui::FontFamily::Name("Noto Serif".into()),
                ),
                CREAM_HEAD,
            );
        }
        if length_galley.size().x > width {
            length_galley = ui.painter().layout(
                length,
                egui::FontId::new(
                    length_size_px,
                    egui::FontFamily::Name("Noto Serif".into()),
                ),
                CREAM_HEAD,
                width,
            );
        }
        let range = format!("{} – {}", clock(clip.start_ms), clock(clip.end_ms));
        let range_galley = ui.painter().layout_no_wrap(
            range,
            egui::FontId::proportional(15.0),
            cream_soft,
        );
        let meta = format!("Clip {} of {total}  ·  {kept} kept", index + 1);
        let meta_galley = ui.painter().layout_no_wrap(
            meta,
            egui::FontId::proportional(13.0),
            MUTED,
        );
        let title_color = if clip.approved { CREAM } else { MUTED };
        let title_font = egui::FontId::proportional(16.0);
        let meta_size = meta_galley.size();
        let title_plain = ui.painter().layout_no_wrap(
            clip.title.clone(),
            title_font.clone(),
            title_color,
        );
        let meta_gap = 20.0;
        let meta_beside = title_plain.size().x + meta_gap + meta_size.x <= width;
        let title_galley = if meta_beside {
            title_plain
        } else {
            ui.painter()
                .layout(clip.title.clone(), title_font, title_color, width)
        };
        let now_size = length_galley.size();
        let range_size = range_galley.size();
        let title_size = title_galley.size();
        let beside_gap = 28.0;
        let range_beside = now_size.x + beside_gap + range_size.x <= width;
        let time_h = if range_beside {
            now_size.y
        } else {
            now_size.y + 4.0 + range_size.y
        };
        let gap_title = 12.0;
        let name_h = if meta_beside {
            title_size.y.max(meta_size.y)
        } else {
            title_size.y + 4.0 + meta_size.y
        };
        let height = time_h + gap_title + name_h;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter();
        painter.galley(rect.left_top(), length_galley, CREAM_HEAD);
        let range_pos = if range_beside {
            egui::pos2(
                rect.left() + now_size.x + beside_gap,
                rect.top() + (now_size.y - range_size.y) * 0.5,
            )
        } else {
            egui::pos2(rect.left(), rect.top() + now_size.y + 4.0)
        };
        painter.galley(range_pos, range_galley, cream_soft);
        let title_y = rect.top() + time_h + gap_title;
        painter.galley(egui::pos2(rect.left(), title_y), title_galley, title_color);
        let meta_pos = if meta_beside {
            egui::pos2(
                rect.left() + title_size.x + meta_gap,
                title_y + (title_size.y - meta_size.y) * 0.5,
            )
        } else {
            egui::pos2(rect.left(), title_y + title_size.y + 4.0)
        };
        painter.galley(meta_pos, meta_galley, MUTED);
    }

    fn review_actions(&mut self, ui: &mut egui::Ui, kept: usize) {
        let width = ui.available_width();
        let export_label = format!("Export {kept} {}", if kept == 1 { "clip" } else { "clips" });
        let export_w = text_button_width(ui, &export_label, 16.0, 22.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.set_min_width(width);
            if review_secondary(ui, "Find again").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Find));
            }
            if review_secondary(ui, "Add a part").clicked() {
                self.dispatch(WizardAction::AddClip);
                self.review_index = self.state.clips.len().saturating_sub(1);
                self.review_for = None;
                self.persist_clips();
                self.focus_review_clip();
            }
            if let Some(clip) = self.review_clip() {
                let keep = if clip.approved {
                    "Drop this one"
                } else {
                    "Keep this one"
                };
                if review_secondary(ui, keep).clicked() {
                    self.toggle_review_clip();
                }
            }
            let spare = ui.available_width() - export_w - 8.0;
            if spare > 8.0 {
                ui.add_space(spare);
            }
            if primary_lg(ui, &export_label).clicked() {
                self.start_export_clips();
            }
        });
    }

    fn export_screen(&mut self, ui: &mut egui::Ui) {
        let done =
            self.state.export_stage == PipelineStage::Done && self.state.output_dir.is_some();
        let failed = self.state.export_error.is_some();
        let title = if done {
            "Export complete"
        } else if failed {
            "Export failed"
        } else {
            "Exporting"
        };
        with_sheet(ui, |ui| {
            let title_size = if ui.available_width() < 520.0 {
                40.0
            } else {
                52.0
            };
            let serif = egui::FontId::new(title_size, egui::FontFamily::Name("Noto Serif".into()));
            ui.label(RichText::new(title).font(serif).color(CREAM_HEAD));
            ui.add_space(16.0);
            if done {
                let folder = self.state.output_dir.clone();
                if let Some(dir) = folder {
                    ui.label(
                        RichText::new(format!("Saved in {}.", file_name(&dir)))
                            .size(18.0)
                            .color(MUTED),
                    );
                    ui.add_space(24.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if primary_lg(ui, "Open folder").clicked() {
                            let _ = backend::open_path(std::path::Path::new(&dir));
                        }
                        if review_secondary(ui, "Back to home").clicked() {
                            self.go_home();
                        }
                    });
                }
            } else if let Some(error) = &self.state.export_error {
                if !error.is_empty() {
                    ui.label(RichText::new(error).size(18.0).color(ORANGE));
                    ui.add_space(24.0);
                }
                if primary_lg(ui, "Back to home").clicked() {
                    self.go_home();
                }
            } else {
                if !self.state.export_message.is_empty() {
                    ui.label(
                        RichText::new(&self.state.export_message)
                            .size(18.0)
                            .color(CREAM),
                    );
                    ui.add_space(20.0);
                }
                export_progress(ui, self.state.export_percent);
                ui.add_space(24.0);
                if secondary(ui, "Cancel").clicked() {
                    self.cancel();
                }
            }
        });
        ui.add_space(80.0);
    }

    fn reframe(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
        self.picture(ui, ctx);
        ui.label(RichText::new("SHAPE OF THE PICTURE").small().color(MUTED));
        ui.label("Drag the frame until the subject sits inside it. Scroll to zoom in. This frame is saved on the video.");
        ui.horizontal(|ui| {
            for preset in CROP_PRESETS {
                if ui
                    .selectable_label(
                        self.ratio == preset.ratio,
                        format!("{}\n{}", preset.label, preset.detail),
                    )
                    .clicked()
                {
                    self.ratio = preset.ratio;
                    if preset.ratio != CropRatio::Original {
                        let crop =
                            self.state
                                .framing
                                .get(&preset.ratio)
                                .copied()
                                .unwrap_or(ClipCrop {
                                    ratio: preset.ratio,
                                    cx: 0.5,
                                    cy: 0.5,
                                    zoom: 0.0,
                                });
                        let mut framing = self.state.framing.clone();
                        framing.insert(preset.ratio, crop);
                        self.dispatch(WizardAction::SetFraming(framing.clone()));
                        if let Some(path) = &self.state.video_path {
                            let _ = sidecars::write_framing(std::path::Path::new(path), &framing);
                        }
                    }
                }
            }
        });
        if self.ratio != CropRatio::Original {
            let mut crop = current_crop(&self.state, self.ratio);
            let mut cx = crop.cx as f32;
            let mut cy = crop.cy as f32;
            let mut zoom = crop.zoom as f32;
            let mut changed = false;
            changed |= ui
                .add(egui::Slider::new(&mut cx, 0.0..=1.0).text("horizontal"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut cy, 0.0..=1.0).text("vertical"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut zoom, 0.0..=1.0).text("zoom"))
                .changed();
            if changed {
                crop.cx = cx as f64;
                crop.cy = cy as f64;
                crop.zoom = zoom as f64;
                crop = crop::snap_crop_center(crop, 1.0, 1.0);
                let mut framing = self.state.framing.clone();
                framing.insert(self.ratio, crop);
                self.dispatch(WizardAction::SetFraming(framing.clone()));
                if let Some(path) = &self.state.video_path {
                    let _ = sidecars::write_framing(std::path::Path::new(path), &framing);
                }
            }
        }
        if self.state.export_stage == PipelineStage::Cutting {
            progress(ui, self.state.export_percent);
        }
        if let Some(error) = &self.state.export_error {
            ui.label(RichText::new(error).color(ORANGE));
        }
        if self.state.export_stage == PipelineStage::Done {
            if let Some(dir) = &self.state.output_dir {
                if primary(ui, "Open folder").clicked() {
                    let _ = backend::open_path(std::path::Path::new(dir));
                }
            }
        } else if primary(ui, "Export this frame").clicked() {
            self.start_reframe();
        }
    }

    fn captions(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.state.video_path.is_none() {
            self.tool_intro(
                ui,
                ToolId::Captions,
                "Captions",
                "Put the words on the picture. You can see the color, size, and font on the video before you export.",
                &[
                    "Pick the video.",
                    "The transcript saved with the video is loaded. Pick another caption file, or edit the words.",
                    "Set the color, position, size, and font, then export.",
                ],
            );
            return;
        }
        self.picture(ui, ctx);
        let cue = active_cues(&self.state, self.caption_source)
            .into_iter()
            .find(|c| self.playhead_ms >= c.start_ms && self.playhead_ms < c.end_ms);
        if let Some(cue) = cue {
            ui.label(
                RichText::new(&cue.text)
                    .size(22.0)
                    .color(ink_color(&self.caption_style)),
            );
        }
        ui.label(
            RichText::new("WHERE DO THE WORDS COME FROM?")
                .small()
                .color(MUTED),
        );
        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.caption_source == CaptionSource::Transcript,
                    "Transcript",
                )
                .clicked()
            {
                self.caption_source = CaptionSource::Transcript;
            }
            if ui
                .selectable_label(self.caption_source == CaptionSource::Manual, "Caption file")
                .clicked()
            {
                self.caption_source = CaptionSource::Manual;
            }
        });
        if self.caption_source == CaptionSource::Manual
            && ui.button("Choose a caption file").clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Captions", &["srt", "txt", "vtt"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let cues = parse_loose_cues(&text);
                        if cues.is_empty() {
                            self.status = "That file did not contain any cues.".into();
                        } else {
                            self.dispatch(WizardAction::SetSegments(cues.clone()));
                            self.status.clear();
                            let project = caption_project(
                                self.caption_source,
                                self.caption_look,
                                self.caption_style.clone(),
                                cues,
                                Some(path.display().to_string()),
                            );
                            self.save_captions(project);
                        }
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
        }
        style_controls(ui, &mut self.caption_style, &mut self.caption_look);
        if ui.button("Fix the words").clicked() {
            self.dispatch(WizardAction::ShowScreen(Screen::FixWords));
        }
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.caption_look == CaptionLook::Srt, "Sidecar .srt")
                .clicked()
            {
                self.caption_look = CaptionLook::Srt;
            }
            if ui
                .selectable_label(
                    self.caption_look == CaptionLook::Burn,
                    "Burn into the picture",
                )
                .clicked()
            {
                self.caption_look = CaptionLook::Burn;
            }
        });
        if !self.status.is_empty() {
            ui.label(RichText::new(&self.status).color(ORANGE));
        }
        if self.state.export_stage == PipelineStage::Cutting {
            progress(ui, self.state.export_percent);
        }
        if let Some(error) = &self.state.export_error {
            ui.label(RichText::new(error).color(ORANGE));
        }
        if self.state.export_stage == PipelineStage::Done {
            if let Some(dir) = &self.state.output_dir {
                if primary(ui, "Open folder").clicked() {
                    let _ = backend::open_path(std::path::Path::new(dir));
                }
            }
        } else if primary(ui, "Export captions").clicked() {
            let cues = active_cues(&self.state, self.caption_source);
            self.save_captions(caption_project(
                self.caption_source,
                self.caption_look,
                self.caption_style.clone(),
                cues,
                None,
            ));
            self.start_captions();
        }
    }

    fn fix_words(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("Fix the words")
                .heading()
                .size(32.0)
                .color(CREAM),
        );
        ui.label("Ask an agent to correct names and phrases. You can accept or reject the edit.");
        if self.agents.is_empty() {
            ui.label("None of grok, claude, codex, or agy is installed on this computer.");
        }
        ui.horizontal(|ui| {
            for (id, label) in self.agents.clone() {
                if ui
                    .selectable_label(self.agent_id.as_deref() == Some(id), label)
                    .clicked()
                {
                    self.agent_id = Some(id.to_string());
                }
            }
        });
        ui.label("What should change?");
        ui.text_edit_multiline(&mut self.fix_request);
        if primary(ui, "Ask").clicked() {
            self.start_fix();
        }
        if self.job.is_some() && ui.button("Cancel").clicked() {
            self.cancel();
        }
        if !self.fix_log.is_empty() {
            ui.label(&self.fix_log);
        }
        if let Some(lines) = self.pending_lines.clone() {
            ui.label(RichText::new("NEW WORDS").small().color(MUTED));
            for line in &lines {
                ui.label(&line.text);
            }
            ui.horizontal(|ui| {
                if primary(ui, "Accept").clicked() {
                    self.dispatch(WizardAction::SetSegments(lines.clone()));
                    self.save_captions(caption_project(
                        self.caption_source,
                        self.caption_look,
                        self.caption_style.clone(),
                        lines,
                        None,
                    ));
                    self.pending_lines = None;
                    self.dispatch(WizardAction::ShowScreen(Screen::Captions));
                }
                if ui.button("Reject").clicked() {
                    self.pending_lines = None;
                }
            });
        }
        if ui.button("Back to captions").clicked() {
            self.dispatch(WizardAction::ShowScreen(Screen::Captions));
        }
    }

    fn store_crop(&mut self, crop: ClipCrop) {
        let mut framing = self.state.framing.clone();
        framing.insert(crop.ratio, crop);
        self.dispatch(WizardAction::SetFraming(framing.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_framing(std::path::Path::new(path), &framing);
        }
    }

    fn save_captions(&mut self, project: CaptionProject) {
        self.dispatch(WizardAction::SetCaptions(project.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_captions(std::path::Path::new(path), &project);
        }
    }

    fn picture(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.picture_limited(ui, ctx, ui.available_width(), 460.0);
    }

    fn picture_limited(
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
                    dragged = Some((
                        stage_response.drag_delta(),
                        image.width(),
                        image.height(),
                    ));
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

    fn play_control(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
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

    fn review_overlay(&mut self, ui: &mut egui::Ui, image: egui::Rect, bar: egui::Rect) {
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

    fn review_transport(
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

    fn review_clock(
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

    fn drag_review_clock(
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

    fn review_seek(
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

    fn paint_review_handles(
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

    fn drag_review_handle(
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

    fn player_volume(&mut self, ui: &mut egui::Ui, speaker: egui::Rect, slider: egui::Rect) {
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

struct FrameResult {
    at_ms: i64,
    image: Result<(u32, u32, Vec<u8>), String>,
}

struct TransportSlots {
    play: egui::Rect,
    seek: egui::Rect,
    time: egui::Rect,
    speaker: egui::Rect,
    slider: egui::Rect,
}

struct SeekHit {
    ms: i64,
    dragging: bool,
}

fn transport_slots(bar: egui::Rect, time_width: f32) -> TransportSlots {
    let pad = 10.0;
    let gap = 8.0;
    let play_s = 40.0_f32.min((bar.height() - 8.0).max(24.0));
    let speaker_s = 36.0_f32.min(play_s);
    let full_slider = 68.0;
    let min_seek = 48.0;
    let with_slider = pad * 2.0
        + play_s
        + gap
        + min_seek
        + gap
        + time_width
        + gap
        + speaker_s
        + gap
        + full_slider;
    let slider_w = if bar.width() >= with_slider {
        full_slider
    } else {
        0.0
    };
    let mid = bar.center().y;
    let mut right = bar.right() - pad;
    let slider = if slider_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - slider_w, mid - 14.0),
            egui::pos2(right, mid + 14.0),
        );
        right = rect.left() - 4.0;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let speaker = egui::Rect::from_center_size(
        egui::pos2(right - speaker_s * 0.5, mid),
        egui::vec2(speaker_s, speaker_s),
    );
    right = speaker.left() - gap;
    let time = egui::Rect::from_min_max(
        egui::pos2(right - time_width, mid - 12.0),
        egui::pos2(right, mid + 12.0),
    );
    let play = egui::Rect::from_center_size(
        egui::pos2(bar.left() + pad + play_s * 0.5, mid),
        egui::vec2(play_s, play_s),
    );
    let seek_left = play.right() + gap;
    let seek = egui::Rect::from_min_max(
        egui::pos2(seek_left, bar.top()),
        egui::pos2((time.left() - gap).max(seek_left), bar.bottom()),
    );
    TransportSlots {
        play,
        seek,
        time,
        speaker,
        slider,
    }
}

struct ReviewSlots {
    play: egui::Rect,
    start_label: egui::Rect,
    seek: egui::Rect,
    end_label: egui::Rect,
    time: egui::Rect,
    speaker: egui::Rect,
    slider: egui::Rect,
}

fn review_slots(bar: egui::Rect, mut start_w: f32, mut end_w: f32, time_w: f32) -> ReviewSlots {
    let pad = 10.0;
    let gap = 8.0;
    let play_s = 40.0_f32.min((bar.height() - 8.0).max(24.0));
    let speaker_s = 36.0_f32.min(play_s);
    let full_slider = 68.0;
    let min_seek = 48.0;
    let label_extra = |start: f32, end: f32| {
        let mut extra = 0.0;
        if start > 0.0 {
            extra += gap + start;
        }
        if end > 0.0 {
            extra += gap + end;
        }
        extra
    };
    let fixed = pad * 2.0 + play_s + gap + min_seek + gap + time_w + gap + speaker_s;
    let fits = |start: f32, end: f32, slider: f32| {
        fixed + label_extra(start, end) + if slider > 0.0 { gap + slider } else { 0.0 }
    };
    let mut slider_w = full_slider;
    if bar.width() < fits(start_w, end_w, slider_w) {
        slider_w = 0.0;
    }
    if bar.width() < fits(start_w, end_w, slider_w) {
        start_w = 0.0;
        end_w = 0.0;
    }
    if slider_w == 0.0 && bar.width() >= fits(start_w, end_w, full_slider) {
        slider_w = full_slider;
    }

    let mid = bar.center().y;
    let mut right = bar.right() - pad;
    let slider = if slider_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - slider_w, mid - 14.0),
            egui::pos2(right, mid + 14.0),
        );
        right = rect.left() - 4.0;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let speaker = egui::Rect::from_center_size(
        egui::pos2(right - speaker_s * 0.5, mid),
        egui::vec2(speaker_s, speaker_s),
    );
    right = speaker.left() - gap;
    let end_label = if end_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - end_w, mid - 12.0),
            egui::pos2(right, mid + 12.0),
        );
        right = rect.left() - gap;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let play = egui::Rect::from_center_size(
        egui::pos2(bar.left() + pad + play_s * 0.5, mid),
        egui::vec2(play_s, play_s),
    );
    let mut left = play.right() + gap;
    let time = egui::Rect::from_min_max(
        egui::pos2(left, mid - 12.0),
        egui::pos2(left + time_w, mid + 12.0),
    );
    left = time.right() + gap;
    let start_label = if start_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(left, mid - 12.0),
            egui::pos2(left + start_w, mid + 12.0),
        );
        left = rect.right() + gap;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(left, mid), egui::pos2(left, mid))
    };
    let seek = egui::Rect::from_min_max(
        egui::pos2(left, bar.top()),
        egui::pos2(right.max(left), bar.bottom()),
    );
    ReviewSlots {
        play,
        start_label,
        seek,
        end_label,
        time,
        speaker,
        slider,
    }
}

fn mono_clock_width(ui: &egui::Ui, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let digit = (0..10)
        .map(|n| {
            ui.painter()
                .layout_no_wrap(n.to_string(), font.clone(), CREAM_HEAD)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let colon = ui
        .painter()
        .layout_no_wrap(":".to_string(), font, CREAM_HEAD)
        .size()
        .x;
    clock(duration_ms).chars().fold(0.0, |width, ch| {
        width + if ch.is_ascii_digit() { digit } else { colon }
    })
}

fn player_time_width(ui: &egui::Ui, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let slash = ui
        .painter()
        .layout_no_wrap(" / ".to_string(), font, CREAM_HEAD)
        .size()
        .x;
    (mono_clock_width(ui, duration_ms) * 2.0 + slash).ceil()
}

fn edge_label_width(ui: &egui::Ui, prefix: &str, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let prefix_w = ui
        .painter()
        .layout_no_wrap(prefix.to_string(), font, CREAM_HEAD)
        .size()
        .x;
    (prefix_w + mono_clock_width(ui, duration_ms)).ceil()
}

fn player_seek(
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

fn level_span(rect: egui::Rect) -> (f32, f32) {
    let left = rect.left() + 7.0;
    let right = (rect.right() - 7.0).max(left + 4.0);
    (left, right)
}

fn paint_level(painter: &egui::Painter, rect: egui::Rect, ratio: f32, hot: bool, fill: Color32) {
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

fn paint_player_time(ui: &egui::Ui, rect: egui::Rect, playhead_ms: i64, duration_ms: i64) {
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

fn paint_clock(ui: &egui::Ui, rect: egui::Rect, playhead_ms: i64) {
    let galley = ui.painter().layout_no_wrap(
        clock(playhead_ms),
        egui::FontId::proportional(13.0),
        CREAM_HEAD,
    );
    let y = rect.center().y - galley.size().y * 0.5;
    ui.painter()
        .galley(egui::pos2(rect.left(), y), galley, CREAM_HEAD);
}

fn paint_edge_label(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(13.0),
        CREAM_HEAD,
    );
    let y = rect.center().y - galley.size().y * 0.5;
    ui.painter()
        .galley(egui::pos2(rect.left(), y), galley, CREAM_HEAD);
}

fn ms_to_x(ms: i64, view: ClipViewWindow, left: f32, right: f32) -> f32 {
    let span = (view.view_end - view.view_start).max(1) as f32;
    let t = (ms - view.view_start) as f32 / span;
    egui::lerp(left..=right, t.clamp(0.0, 1.0))
}

fn paint_clip_track(
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

fn paint_chevron(painter: &egui::Painter, rect: egui::Rect, left: bool) {
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

struct ReviewFrame {
    picture: egui::Vec2,
    button: f32,
    gap: f32,
}

fn review_frame(avail_w: f32, avail_h: f32, tex_w: f32, tex_h: f32, dock: f32) -> ReviewFrame {
    let button = if avail_w < 720.0 { 56.0 } else { 76.0 };
    let gap = 24.0;
    let margin = 24.0;
    // Leave the measured dock on screen, plus a couple of pixels so a scrollbar does not appear.
    let max_w = (avail_w - 2.0 * (button + gap + margin)).max(160.0);
    let max_h = (avail_h - dock - 2.0).max(180.0);
    ReviewFrame {
        picture: fit_picture(tex_w, tex_h, max_w, max_h),
        button,
        gap,
    }
}

fn fit_picture(tex_w: f32, tex_h: f32, max_w: f32, max_h: f32) -> egui::Vec2 {
    let tex_w = tex_w.max(1.0);
    let tex_h = tex_h.max(1.0);
    let max_w = max_w.max(1.0);
    let max_h = max_h.max(1.0);
    let mut width = max_w;
    let mut height = width * tex_h / tex_w;
    if height > max_h {
        height = max_h;
        width = height * tex_w / tex_h;
    }
    egui::vec2(width, height)
}

/// After a start or end drag, playback stays inside the clip and the loop stays on.
fn playhead_after_trim(
    resume_ms: i64,
    preview_ms: i64,
    start_ms: i64,
    end_ms: i64,
    playing: bool,
) -> i64 {
    let end_ms = end_ms.max(start_ms);
    if playing {
        if resume_ms >= start_ms && resume_ms < end_ms {
            resume_ms
        } else {
            start_ms
        }
    } else if preview_ms >= end_ms {
        end_ms.saturating_sub(1).max(start_ms)
    } else if preview_ms < start_ms {
        start_ms
    } else {
        preview_ms
    }
}

fn clip_length_words(start_ms: i64, end_ms: i64) -> String {
    let total = (end_ms - start_ms).max(0) / 1000;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(count_words(hours, "hour", "hours"));
    }
    if minutes > 0 {
        parts.push(count_words(minutes, "minute", "minutes"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(count_words(seconds, "second", "seconds"));
    }
    match parts.len() {
        1 => parts.remove(0),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().expect("at least one part");
            format!("{} and {last}", parts.join(", "))
        }
    }
}

fn count_words(count: i64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn text_button_width(ui: &egui::Ui, label: &str, text_size: f32, pad_x: f32) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(text_size),
        Color32::WHITE,
    );
    galley.size().x + pad_x * 2.0
}

fn paint_seek_tip(
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

fn paint_player_scrim(painter: &egui::Painter, rect: egui::Rect) {
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

fn paint_icon_hover(painter: &egui::Painter, response: &egui::Response) {
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

fn paint_play_icon(painter: &egui::Painter, rect: egui::Rect) {
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

fn paint_pause_icon(painter: &egui::Painter, rect: egui::Rect) {
    let c = rect.center();
    for dx in [-3.6_f32, 3.6] {
        let bar = egui::Rect::from_center_size(c + egui::vec2(dx, 0.0), egui::vec2(3.2, 14.0));
        painter.rect_filled(bar, 1.0, CREAM_HEAD);
    }
}

fn paint_speaker(painter: &egui::Painter, rect: egui::Rect, muted: bool, volume: f32) {
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

fn apply_theme(ctx: &egui::Context) {
    install_type(ctx);
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = CANVAS;
    visuals.panel_fill = CANVAS;
    visuals.extreme_bg_color = Color32::from_rgb(0x14, 0x14, 0x14);
    visuals.faint_bg_color = SURFACE;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, CREAM);
    visuals.widgets.inactive.bg_fill = SURFACE;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x1c, 0x18);
    visuals.selection.bg_fill = ORANGE;
    visuals.hyperlink_color = ORANGE;
    visuals.widgets.active.bg_fill = ORANGE;
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.visuals.override_text_color = Some(CREAM);
    });
}

fn install_type(ctx: &egui::Context) {
    let sans = "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf";
    let serif = "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf";
    if let Ok(bytes) = std::fs::read(sans) {
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "Noto Sans",
            egui::FontData::from_owned(bytes),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Proportional,
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
    }
    if let Ok(bytes) = std::fs::read(serif) {
        ctx.add_font(egui::epaint::text::FontInsert::new(
            "Noto Serif",
            egui::FontData::from_owned(bytes),
            vec![egui::epaint::text::InsertFontFamily {
                family: egui::FontFamily::Name("Noto Serif".into()),
                priority: egui::epaint::text::FontPriority::Highest,
            }],
        ));
    }
}

fn paint_sunset(painter: &egui::Painter, rect: egui::Rect) {
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

fn home_tile(
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
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
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
            ui.label(
                RichText::new(title)
                    .size(if compact { 18.0 } else { 22.0 })
                    .color(CREAM_HEAD),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(detail)
                    .size(if compact { 13.0 } else { 15.0 })
                    .color(MUTED),
            );
        });
    });
    response
}

fn paint_tool_icon(painter: &egui::Painter, tile: egui::Rect, tool: ToolId) {
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

fn with_sheet(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
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

fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    primary_sized(ui, label, 40.0, 14.0)
}

fn primary_lg(ui: &mut egui::Ui, label: &str) -> egui::Response {
    primary_sized(ui, label, 48.0, 16.0)
}

fn secondary(ui: &mut egui::Ui, label: &str) -> egui::Response {
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

fn review_secondary(ui: &mut egui::Ui, label: &str) -> egui::Response {
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

fn danger(ui: &mut egui::Ui, label: &str) -> egui::Response {
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

fn choice(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
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

fn text_hit(ui: &mut egui::Ui, label: &str, size: f32, chevron: bool) -> egui::Response {
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

fn recent_row(
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

fn progress(ui: &mut egui::Ui, percent: f64) {
    let bar = egui::ProgressBar::new((percent as f32 / 100.0).clamp(0.0, 1.0))
        .text(format!("{percent:.0}%"));
    ui.add(bar);
}

fn export_progress(ui: &mut egui::Ui, percent: f64) {
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

fn clock(ms: i64) -> String {
    a1slice_core::preview::format_playback_clock(ms.max(0) as f64)
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

fn settings_of(state: &WizardState) -> AppSettings {
    AppSettings {
        provider: state.provider,
        model: state.model.clone(),
        api_key: state.api_key.clone(),
        api_keys: state.api_keys.clone(),
        user_hint: state.user_hint.clone(),
        language: state.language,
        entropy_thold: state.entropy_thold,
        max_context: state.max_context,
        beam_size: state.beam_size,
        temperature_inc: state.temperature_inc,
    }
}

fn default_style() -> CaptionStyle {
    CaptionStyle {
        color: CaptionColor::White,
        custom_color: None,
        position: CaptionPosition::Bottom,
        size: CaptionSize::Large,
        font_size: None,
        font: CaptionFont::Sans,
    }
}

fn current_crop(state: &WizardState, ratio: CropRatio) -> ClipCrop {
    if ratio == CropRatio::Original {
        DEFAULT_CROP
    } else {
        state.framing.get(&ratio).copied().unwrap_or(ClipCrop {
            ratio,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        })
    }
}

fn active_cues(state: &WizardState, source: CaptionSource) -> Vec<TranscriptSegment> {
    if source == CaptionSource::Manual {
        if let Some(captions) = &state.captions {
            if captions.source == CaptionSource::Manual && !captions.cues.is_empty() {
                return captions.cues.clone();
            }
        }
    }
    state.segments.clone()
}

fn caption_project(
    source: CaptionSource,
    look: CaptionLook,
    style: CaptionStyle,
    cues: Vec<TranscriptSegment>,
    file_path: Option<String>,
) -> CaptionProject {
    CaptionProject {
        source,
        look,
        style,
        cues,
        file_path,
    }
}

fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

fn ink_color(style: &CaptionStyle) -> Color32 {
    if let Some(hex) = style
        .custom_color
        .as_deref()
        .and_then(|h| h.strip_prefix('#'))
    {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color32::from_rgb(r, g, b);
            }
        }
    }
    caption_color(style.color)
}

fn caption_color(color: CaptionColor) -> Color32 {
    match color {
        CaptionColor::White => Color32::WHITE,
        CaptionColor::Cream => Color32::from_rgb(0xff, 0xf8, 0xe0),
        CaptionColor::Yellow => Color32::from_rgb(0xff, 0xe1, 0x4a),
        CaptionColor::Black => Color32::from_rgb(0x11, 0x11, 0x11),
    }
}

fn style_controls(ui: &mut egui::Ui, style: &mut CaptionStyle, _look: &mut CaptionLook) {
    ui.label(RichText::new("COLOR").small().color(MUTED));
    ui.horizontal_wrapped(|ui| {
        for (color, label) in [
            (CaptionColor::White, "White"),
            (CaptionColor::Cream, "Cream"),
            (CaptionColor::Yellow, "Yellow"),
            (CaptionColor::Black, "Black"),
        ] {
            if ui
                .selectable_label(style.color == color && style.custom_color.is_none(), label)
                .clicked()
            {
                style.color = color;
                style.custom_color = None;
            }
        }
    });
    let mut custom = style.custom_color.clone().unwrap_or_default();
    if ui.text_edit_singleline(&mut custom).changed() {
        if custom.trim().is_empty() {
            style.custom_color = None;
        } else if let Some(hex) = normalize_hex(&custom) {
            style.custom_color = Some(hex);
        }
    }
    ui.label(
        RichText::new("Or type a colour, like #ffb83e.")
            .small()
            .color(MUTED),
    );
    ui.label(RichText::new("POSITION").small().color(MUTED));
    ui.horizontal(|ui| {
        for (pos, label) in [
            (CaptionPosition::Bottom, "Bottom"),
            (CaptionPosition::Middle, "Middle"),
            (CaptionPosition::Top, "Top"),
        ] {
            if ui.selectable_label(style.position == pos, label).clicked() {
                style.position = pos;
            }
        }
    });
    ui.label(RichText::new("SIZE").small().color(MUTED));
    ui.horizontal(|ui| {
        for (size, label) in [
            (CaptionSize::Small, "Small"),
            (CaptionSize::Medium, "Medium"),
            (CaptionSize::Large, "Large"),
        ] {
            if ui.selectable_label(style.size == size, label).clicked() {
                style.size = size;
                style.font_size = None;
            }
        }
    });
    ui.label(RichText::new("FONT").small().color(MUTED));
    ui.horizontal(|ui| {
        for (font, label) in [
            (CaptionFont::Sans, "Sans"),
            (CaptionFont::Serif, "Serif"),
            (CaptionFont::Mono, "Mono"),
        ] {
            if ui.selectable_label(style.font == font, label).clicked() {
                style.font = font;
            }
        }
    });
}

fn load_recent() -> Vec<(String, String)> {
    sidecars::read_recent()
        .into_iter()
        .map(|path| {
            let info = sidecars::inspect_video(std::path::Path::new(&path));
            let mut parts = Vec::new();
            if info.has_transcript {
                parts.push("Transcript saved".to_string());
            }
            if info.has_clips {
                parts.push(if info.clip_count == 1 {
                    "1 clip".into()
                } else {
                    format!("{} clips", info.clip_count)
                });
            }
            if info.has_framing {
                parts.push("Frame saved".into());
            }
            if info.has_captions {
                parts.push("Captions saved".into());
            }
            (path, parts.join(" · "))
        })
        .collect()
}

fn parse_loose_cues(text: &str) -> Vec<TranscriptSegment> {
    // SRT blocks, then the plain transcript lines this app writes.
    let mut cues = Vec::new();
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let Some(time) = lines.iter().find(|l| l.contains("-->")) else {
            continue;
        };
        let Some((start, end)) = time.split_once("-->") else {
            continue;
        };
        let start_ms = srt_clock(start.trim());
        let end_ms = srt_clock(end.trim());
        let body = lines
            .iter()
            .skip_while(|l| !l.contains("-->"))
            .skip(1)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        if end_ms > start_ms && !body.is_empty() {
            cues.push(TranscriptSegment {
                start_ms,
                end_ms,
                text: body,
            });
        }
    }
    if !cues.is_empty() {
        return cues;
    }
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        let mut parts = line.splitn(2, "  ");
        let stamp = parts.next().unwrap_or("");
        let text = parts.next().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        rows.push((loose_clock(stamp), text.to_string()));
    }
    rows.iter()
        .enumerate()
        .map(|(i, (start, text))| {
            let end = rows
                .get(i + 1)
                .map(|(n, _)| *n)
                .filter(|n| *n > *start)
                .unwrap_or(start + 2000);
            TranscriptSegment {
                start_ms: *start,
                end_ms: end,
                text: text.clone(),
            }
        })
        .collect()
}

fn srt_clock(text: &str) -> i64 {
    let text = text.replace(',', ".");
    let mut parts = text.split(':');
    let h: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let m: i64 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let rest = parts.next().unwrap_or("0");
    let mut sec = rest.split('.');
    let s: i64 = sec.next().unwrap_or("0").parse().unwrap_or(0);
    let frac = sec.next().unwrap_or("0");
    let millis: i64 = format!("{frac:0<3}")
        .chars()
        .take(3)
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    h * 3_600_000 + m * 60_000 + s * 1000 + millis
}

fn loose_clock(text: &str) -> i64 {
    let bits: Vec<&str> = text.split(':').collect();
    match bits.as_slice() {
        [m, s] => m.parse::<i64>().unwrap_or(0) * 60_000 + s.parse::<i64>().unwrap_or(0) * 1000,
        [h, m, s] => {
            h.parse::<i64>().unwrap_or(0) * 3_600_000
                + m.parse::<i64>().unwrap_or(0) * 60_000
                + s.parse::<i64>().unwrap_or(0) * 1000
        }
        _ => 0,
    }
}

fn start_audio(path: &str, at_ms: i64, volume: f32) -> Option<std::process::Child> {
    let sec = format!("{:.3}", at_ms.max(0) as f64 / 1000.0);
    let vol = ((volume.clamp(0.0, 1.0)) * 100.0) as i32;
    if backend::which("ffplay").is_some() {
        return std::process::Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-ss",
                &sec,
                "-volume",
                &vol.to_string(),
                "-loglevel",
                "quiet",
                path,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    }
    if backend::which("mpv").is_some() {
        return std::process::Command::new("mpv")
            .args([
                "--no-video",
                "--really-quiet",
                &format!("--start={sec}"),
                &format!("--volume={vol}"),
                path,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        clip_length_words, fit_picture, ms_to_x, playhead_after_trim, review_frame, review_slots,
        transport_slots, ClipViewWindow,
    };

    #[test]
    fn transport_row_is_play_seek_time_then_volume() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 56.0));
        let slots = transport_slots(bar, 72.0);
        assert!(slots.play.right() < slots.seek.left());
        assert!(slots.seek.right() <= slots.time.left());
        assert!(slots.time.right() < slots.speaker.left());
        assert!(slots.speaker.right() < slots.slider.left());
        assert!(slots.slider.width() > 0.0);
        assert!(slots.seek.width() > 200.0);
        assert!(slots.play.left() >= bar.left());
        assert!(slots.slider.right() <= bar.right());
        let mid = bar.center().y;
        for center in [
            slots.play.center().y,
            slots.seek.center().y,
            slots.time.center().y,
            slots.speaker.center().y,
            slots.slider.center().y,
        ] {
            assert!((center - mid).abs() < 0.5);
        }
    }

    #[test]
    fn review_row_places_handles_between_the_times() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 56.0));
        let slots = review_slots(bar, 80.0, 70.0, 48.0);
        assert!(slots.play.right() < slots.time.left());
        assert!(slots.time.right() <= slots.start_label.left());
        assert!(slots.start_label.right() <= slots.seek.left());
        assert!(slots.seek.right() <= slots.end_label.left());
        assert!(slots.end_label.right() < slots.speaker.left());
        assert!(slots.speaker.right() < slots.slider.left());
        assert!(slots.seek.width() > 48.0);
        assert!(slots.slider.width() > 0.0);
    }

    #[test]
    fn narrow_review_row_drops_the_time_labels() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(280.0, 56.0));
        let slots = review_slots(bar, 80.0, 70.0, 48.0);
        assert_eq!(slots.start_label.width(), 0.0);
        assert_eq!(slots.end_label.width(), 0.0);
        assert_eq!(slots.slider.width(), 0.0);
        assert!(slots.seek.width() > 8.0);
        assert!(slots.play.right() < slots.time.left());
        assert!(slots.time.right() <= slots.seek.left());
        assert!(slots.seek.right() <= slots.speaker.left());
    }

    #[test]
    fn review_picture_grows_with_the_window() {
        let wide = review_frame(1600.0, 900.0, 1920.0, 1080.0, 248.0);
        assert!(wide.picture.y > 460.0);
        assert!(wide.picture.x > 960.0);
        assert!(wide.button >= 72.0);
        let row = wide.button * 2.0 + wide.gap * 2.0 + wide.picture.x;
        assert!(row < 1600.0);
        let narrow = review_frame(1100.0, 700.0, 1920.0, 1080.0, 248.0);
        assert!(wide.picture.x > narrow.picture.x);
        assert!(wide.picture.y > narrow.picture.y);
        let fitted = fit_picture(1920.0, 1080.0, wide.picture.x, wide.picture.y);
        assert!((fitted.x - wide.picture.x).abs() < 0.5);
        assert!((fitted.y - wide.picture.y).abs() < 0.5);
    }

    #[test]
    fn dragging_the_end_keeps_playback_inside_the_clip() {
        let start = 3_460;
        let end = 25_710;
        let resume = 12_000;
        let preview = end;
        assert_eq!(
            playhead_after_trim(resume, preview, start, end, true),
            resume
        );
        let pulled_in = 8_000;
        assert_eq!(
            playhead_after_trim(resume, pulled_in, start, pulled_in, true),
            start
        );
        assert_eq!(
            playhead_after_trim(resume, preview, start, end, false),
            end - 1
        );
        assert!(playhead_after_trim(resume, preview, start, end, false) < end);
    }

    #[test]
    fn clip_handle_sits_on_its_time() {
        let view = ClipViewWindow {
            view_start: 0,
            view_end: 10_000,
        };
        let start = ms_to_x(2_000, view, 0.0, 100.0);
        let end = ms_to_x(8_000, view, 0.0, 100.0);
        assert!((start - 20.0).abs() < 0.1);
        assert!((end - 80.0).abs() < 0.1);
        assert!(start < end);
    }

    #[test]
    fn narrow_transport_keeps_mute_and_drops_the_slider() {
        let bar = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(280.0, 56.0));
        let slots = transport_slots(bar, 90.0);
        assert_eq!(slots.slider.width(), 0.0);
        assert!(slots.play.right() < slots.seek.left());
        assert!(slots.seek.right() <= slots.time.left());
        assert!(slots.time.right() < slots.speaker.left());
        assert!(slots.speaker.right() <= bar.right());
        assert!(slots.seek.width() > 8.0);
    }

    #[test]
    fn clip_length_is_spoken_in_words() {
        assert_eq!(clip_length_words(0, 0), "0 seconds");
        assert_eq!(clip_length_words(0, 1_000), "1 second");
        assert_eq!(clip_length_words(0, 59_000), "59 seconds");
        assert_eq!(clip_length_words(0, 60_000), "1 minute");
        assert_eq!(clip_length_words(0, 75_000), "1 minute and 15 seconds");
        assert_eq!(clip_length_words(0, 121_000), "2 minutes and 1 second");
        assert_eq!(clip_length_words(0, 3_600_000), "1 hour");
        assert_eq!(
            clip_length_words(0, 3_600_000 + 15_000),
            "1 hour and 15 seconds"
        );
        assert_eq!(
            clip_length_words(0, 7_200_000 + 120_000 + 5_000),
            "2 hours, 2 minutes and 5 seconds"
        );
    }
}
