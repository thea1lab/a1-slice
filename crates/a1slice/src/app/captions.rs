//! Captions screen and Fix the words.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, RichText};

use super::support::{active_cues, caption_project, ink_color, style_controls};
use super::theme::{CREAM, MUTED, ORANGE};
use super::widgets::{primary, progress};
use super::A1App;

impl A1App {
    pub(super) fn captions(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
                        let cues = a1slice_core::project::parse_caption_document(&text);
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

    pub(super) fn fix_words(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn save_captions(&mut self, project: CaptionProject) {
        self.dispatch(WizardAction::SetCaptions(project.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_captions(std::path::Path::new(path), &project);
        }
    }
}
