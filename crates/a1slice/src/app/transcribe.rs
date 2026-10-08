//! Transcribe screen and the saved-transcript screen.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, RichText};

use super::support::{clock, file_name};
use super::theme::{CREAM, MUTED, ORANGE};
use super::widgets::{primary, progress};
use super::A1App;

impl A1App {
    pub(super) fn transcribe(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn transcribe_done(&mut self, ui: &mut egui::Ui) {
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
}
