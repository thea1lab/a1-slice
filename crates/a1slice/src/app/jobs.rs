//! Open a video and run transcribe, find, export, reframe, and fix.

use crate::backend::{self, JobDone};
use a1slice_core::sidecars::{self, with_clip_status};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self};
use std::path::PathBuf;
use std::sync::mpsc::{self};
use std::time::Instant;

use super::support::{active_cues, current_crop, load_recent};
use super::{A1App, ReviewTrim};

impl A1App {
    pub(super) fn open_video(&mut self, tool: ToolId, path: PathBuf) {
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

    pub(super) fn choose_video(&mut self, tool: ToolId) {
        let file = rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "avi", "webm"])
            .pick_file();
        if let Some(path) = file {
            self.open_video(tool, path);
        }
    }

    pub(super) fn poll_job(&mut self) {
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

    pub(super) fn start_transcribe(&mut self) {
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

    pub(super) fn start_find(&mut self) {
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

    pub(super) fn start_export_clips(&mut self) {
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

    pub(super) fn start_reframe(&mut self) {
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

    pub(super) fn start_captions(&mut self) {
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

    pub(super) fn start_fix(&mut self) {
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

    pub(super) fn cancel(&mut self) {
        if let Some(job) = &self.job {
            backend::cancel_job(job);
        }
    }

    pub(super) fn tick_find_progress(&mut self, ctx: &egui::Context) {
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
}
