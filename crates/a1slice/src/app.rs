//! The A1 Slice window. Same tools as the Electron app: transcribe, find, reframe, captions.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Instant;

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
    ratio: CropRatio,
    caption_style: CaptionStyle,
    caption_look: CaptionLook,
    caption_source: CaptionSource,
    fix_request: String,
    fix_log: String,
    pending_lines: Option<Vec<TranscriptSegment>>,
    status: String,
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
            ratio: CropRatio::R9x16,
            caption_style: default_style(),
            caption_look: CaptionLook::Burn,
            caption_source: CaptionSource::Transcript,
            fix_request: String::new(),
            fix_log: String::new(),
            pending_lines: None,
            status: String::new(),
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
        if (self.state.analyze_percent - percent).abs() > 0.3 || self.state.analyze_message != message {
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
                let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
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
                self.audio = start_audio(&path, self.playhead_ms, self.volume);
            }
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
            let end = self.state.video_duration_ms.max(1);
            if self.playhead_ms >= end {
                self.playhead_ms = 0;
                self.stop_playback();
            }
            ctx.request_repaint();
        }
        let show_picture = matches!(
            self.state.screen,
            Screen::Review | Screen::Reframe | Screen::Captions | Screen::FixWords
        ) && self.state.video_path.is_some();
        if show_picture {
            self.poll_frame(ctx);
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
            ui.label(RichText::new("Find best parts").font(serif.clone()).color(CREAM_HEAD));
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
                RichText::new(format!("Transcript · {lines} {}", if lines == 1 { "line" } else { "lines" }))
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
                ui.label(RichText::new("This computer has no agent to run. You can still mark a part yourself.").size(16.0).color(MUTED));
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
            ui.label(RichText::new("What should it look for?").size(16.0).color(MUTED));
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
            ui.label(RichText::new("Optional. Name a theme, a number of clips, or a moment to keep.").size(16.0).color(MUTED));
            if self.state.analyzing {
                ui.add_space(16.0);
                progress(ui, self.state.analyze_percent);
                ui.label(RichText::new(&self.state.analyze_message).size(15.0).color(MUTED));
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
                            ui.label(RichText::new("Could not find clips").size(16.0).color(Color32::from_rgb(0xfc, 0xa5, 0xa5)));
                            ui.add_space(6.0);
                            ui.label(RichText::new(error).size(14.0).color(Color32::from_rgb(0xfc, 0xa5, 0xa5)));
                        });
                }
            }
            ui.add_space(16.0);
            if !self.state.analyzing && !self.state.clips.is_empty() && secondary(ui, "Back to the clips").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Review));
            }
            ui.add_space(12.0);
            if self.state.analyzing {
                if danger(ui, "Cancel").clicked() {
                    self.cancel();
                }
            } else {
                let label = if self.state.clips.is_empty() { "Find clips" } else { "Search again" };
                let enabled = self.agent_id.is_some();
                if primary_lg(ui, label).clicked() && enabled {
                    self.start_find();
                }
            }
        });
        ui.add_space(80.0);
    }

    fn review(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let avail = ui.available_width();
        let column = avail.min(960.0);
        let side = ((avail - column) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(column);
                self.review_column(ui, ctx);
            });
        });
    }

    fn review_column(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.picture(ui, ctx);
        ui.add_space(16.0);
        let (hair, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(hair.x_range(), hair.center().y, Stroke::new(1.0_f32, HAIRLINE));
        ui.add_space(16.0);
        let kept = self.state.clips.iter().filter(|clip| clip.approved).count();
        let total = self.state.clips.len();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            ui.label(RichText::new(format!("{total} {}", if total == 1 { "clip" } else { "clips" })).size(15.0).color(MUTED));
            ui.label(RichText::new(format!("{kept} kept")).size(15.0).color(MUTED));
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Turn a clip off to leave it out. Nudge the start and end, then export.").size(16.0).color(MUTED));
        ui.add_space(16.0);
        let ids: Vec<String> = self.state.clips.iter().map(|clip| clip.id.clone()).collect();
        for id in ids {
            let Some(clip) = self.state.clips.iter().find(|clip| clip.id == id).cloned() else {
                continue;
            };
            ui.label(RichText::new(&clip.title).size(18.0).color(if clip.approved { CREAM_HEAD } else { MUTED }));
            ui.label(RichText::new(format!("{} – {}", clock(clip.start_ms), clock(clip.end_ms))).size(15.0).color(MUTED));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                let keep = if clip.approved { "Drop this one" } else { "Keep this one" };
                if secondary(ui, keep).clicked() {
                    self.dispatch(WizardAction::ToggleClip(id.clone()));
                }
                for (label, edge, delta) in [("Start −1s", "start", -1000.0), ("Start +1s", "start", 1000.0), ("End −1s", "end", -1000.0), ("End +1s", "end", 1000.0)] {
                    if secondary(ui, label).clicked() {
                        let (start, end) = a1slice_core::clips::nudge_edge(clip.start_ms, clip.end_ms, edge, delta, self.state.video_duration_ms as f64);
                        self.dispatch(WizardAction::UpdateClipTimes { id: id.clone(), start_ms: start, end_ms: end });
                    }
                }
            });
            ui.add_space(16.0);
        }
        if !self.status.is_empty() {
            ui.label(RichText::new(&self.status).color(ORANGE));
            ui.add_space(8.0);
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            if secondary(ui, "Find again").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Find));
            }
            if secondary(ui, "Add a part").clicked() {
                self.dispatch(WizardAction::AddClip);
            }
            let export_label = format!("Export {kept} {}", if kept == 1 { "clip" } else { "clips" });
            if primary_lg(ui, &export_label).clicked() {
                self.start_export_clips();
            }
        });
        ui.add_space(80.0);
    }

    fn export_screen(&mut self, ui: &mut egui::Ui) {
        let done =
            self.state.export_stage == PipelineStage::Done && self.state.output_dir.is_some();
        let title = if done {
            "Export complete"
        } else if self.state.export_error.is_some() {
            "Export failed"
        } else {
            "Exporting"
        };
        ui.label(RichText::new(title).heading().size(36.0).color(CREAM));
        if done {
            if let Some(dir) = &self.state.output_dir {
                ui.label(format!("Saved in {}.", file_name(dir)));
                if primary(ui, "Open folder").clicked() {
                    let _ = backend::open_path(std::path::Path::new(dir));
                }
            }
            if ui.button("Reframe").clicked() {
                if let Some(path) = self.state.video_path.clone() {
                    self.open_video(ToolId::Reframe, PathBuf::from(path));
                }
            }
            if ui.button("Captions").clicked() {
                if let Some(path) = self.state.video_path.clone() {
                    self.open_video(ToolId::Captions, PathBuf::from(path));
                }
            }
            if ui.button("Back to home").clicked() {
                self.go_home();
            }
        } else if let Some(error) = &self.state.export_error {
            ui.label(RichText::new(error).color(ORANGE));
            if ui.button("Back to home").clicked() {
                self.go_home();
            }
        } else {
            progress(ui, self.state.export_percent);
            ui.label(&self.state.export_message);
            if ui.button("Cancel").clicked() {
                self.cancel();
            }
        }
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

    fn picture(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let mut dragged = None;
        let mut scrolled = 0.0_f32;
        if let Some(texture) = &self.frame {
            let size = texture.size_vec2();
            let max_w = ui.available_width();
            let mut width = max_w;
            let mut height = width * size.y / size.x.max(1.0);
            if height > 460.0 {
                height = 460.0;
                width = height * size.x / size.y.max(1.0);
            }
            let pad = ((max_w - width) * 0.5).max(0.0);
            let response = ui.horizontal(|ui| {
                ui.add_space(pad);
                ui.image((texture.id(), egui::vec2(width, height)))
            }).inner;
            if response.clicked() {
                self.toggle_play();
            }
            if response.dragged() {
                dragged = Some((response.drag_delta(), width, height));
            }
            if response.hovered() {
                scrolled = ui.input(|i| i.smooth_scroll_delta.y);
                ui.ctx().set_cursor_icon(if self.playing { egui::CursorIcon::Default } else { egui::CursorIcon::PointingHand });
            }
        } else {
            ui.label(RichText::new("Reading a frame…").color(MUTED));
        }
        if self.state.screen == Screen::Reframe && self.ratio != CropRatio::Original {
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
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            let label = if self.playing { "Pause" } else { "Play" };
            if secondary(ui, label).clicked() {
                self.toggle_play();
            }
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    clock(self.playhead_ms),
                    clock(self.state.video_duration_ms)
                ))
                .size(15.0)
                .color(MUTED),
            );
            let mut volume = self.volume;
            if ui
                .add_sized(
                    [140.0, 18.0],
                    egui::Slider::new(&mut volume, 0.0..=1.0).text("Volume").show_value(false),
                )
                .changed()
            {
                self.volume = volume;
                if self.playing {
                    self.stop_audio();
                    if let Some(path) = self.state.video_path.clone() {
                        self.audio = start_audio(&path, self.playhead_ms, self.volume);
                    }
                }
            }
        });
        if let Some(ms) = seek_bar(ui, self.playhead_ms, self.state.video_duration_ms) {
            self.playhead_ms = ms;
            self.scrubbing = true;
        } else if self.scrubbing && ui.input(|i| i.pointer.any_released()) {
            self.scrubbing = false;
            self.restart_audio();
        }
    }
}

struct FrameResult {
    at_ms: i64,
    image: Result<(u32, u32, Vec<u8>), String>,
}

fn seek_bar(ui: &mut egui::Ui, playhead_ms: i64, duration_ms: i64) -> Option<i64> {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click_and_drag());
    let duration = duration_ms.max(1) as f32;
    let mut ratio = (playhead_ms.max(0) as f32 / duration).clamp(0.0, 1.0);
    let interacting = response.dragged() || response.clicked();
    if interacting {
        if let Some(pointer) = response.interact_pointer_pos() {
            ratio = ((pointer.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        }
    }
    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), 4.0));
    ui.painter().rect_filled(track, 2.0, Color32::from_rgb(0x2a, 0x2a, 0x2a));
    let mut filled = track;
    filled.set_right(track.left() + track.width() * ratio);
    ui.painter().rect_filled(filled, 2.0, ORANGE);
    let handle = egui::pos2(filled.right(), track.center().y);
    ui.painter().circle_filled(handle, if response.hovered() || interacting { 7.0 } else { 5.5 }, CREAM_HEAD);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    interacting.then_some((ratio * duration) as i64)
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
    let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(16.0), color);
    let size = egui::vec2((galley.size().x + 32.0).max(72.0), 44.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let border = if selected {
        ORANGE
    } else if response.hovered() {
        Color32::from_rgb(0x8a, 0x8a, 0x8a)
    } else {
        Color32::from_rgb(0x3a, 0x3a, 0x3a)
    };
    let fill = if selected { Color32::from_rgb(0x24, 0x1c, 0x18) } else { SURFACE };
    ui.painter().rect(rect, egui::CornerRadius::same(8), fill, Stroke::new(1.0_f32, border), egui::StrokeKind::Inside);
    let pos = egui::pos2(rect.center().x - galley.size().x * 0.5, rect.center().y - galley.size().y * 0.5);
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
    let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(text_size), text_color);
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
    let pos = egui::pos2(rect.center().x - galley.size().x * 0.5, rect.center().y - galley.size().y * 0.5);
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
