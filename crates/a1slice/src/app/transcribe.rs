//! Transcribe screen and the saved-transcript screen.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::support::clock;
use super::theme::{CREAM, CREAM_HEAD, HAIRLINE, MUTED, ORANGE, SURFACE};
use super::widgets::{choice, export_progress, primary_lg, review_secondary, with_sheet};
use super::A1App;

fn sheet_title(ui: &mut egui::Ui, title: &str) {
    let title_size = if ui.available_width() < 520.0 {
        40.0
    } else {
        52.0
    };
    let serif = egui::FontId::new(title_size, egui::FontFamily::Name("Noto Serif".into()));
    ui.label(RichText::new(title).font(serif).color(CREAM_HEAD));
}

fn shown_number(value: f32, decimals: usize) -> String {
    if decimals == 0 {
        return format!("{}", value.round() as i64);
    }
    format!("{value:.decimals$}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureKind {
    Model,
    Program,
    Other,
}

fn failure_kind(error: &str) -> FailureKind {
    if error.contains("Whisper model is not") {
        FailureKind::Model
    } else if error.contains("whisper program is not built") {
        FailureKind::Program
    } else {
        FailureKind::Other
    }
}

fn micro(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(11.0)
            .strong()
            .color(MUTED)
            .extra_letter_spacing(1.0),
    );
}

fn file_path(ui: &mut egui::Ui, path: &str) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, HAIRLINE))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(path).size(15.0).color(CREAM));
        });
}

fn failure_card(ui: &mut egui::Ui, error: &str) {
    let ink = Color32::from_rgb(0xfc, 0xa5, 0xa5);
    egui::Frame::new()
        .fill(Color32::from_rgb(0x3a, 0x20, 0x1c))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x7a, 0x3b, 0x32)))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(16.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new(error).size(16.0).color(ink));
        });
}

fn setting_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    slider: egui::Slider<'_>,
) -> egui::Response {
    ui.add_space(14.0);
    ui.label(RichText::new(label).size(16.0).color(CREAM));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        ui.spacing_mut().slider_width = (ui.available_width() - 48.0).max(80.0);
        let response = ui.add(slider.show_value(false).trailing_fill(true));
        ui.label(RichText::new(value).size(15.0).color(CREAM));
        response
    })
    .inner
}

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
        } else {
            let running = matches!(
                self.state.transcribe_stage,
                PipelineStage::Extracting
                    | PipelineStage::Transcribing
                    | PipelineStage::Downloading
            );
            let failed = self.state.transcribe_stage == PipelineStage::Error;
            with_sheet(ui, |ui| {
                if running {
                    self.transcribe_running(ui);
                } else if failed {
                    self.transcribe_failed(ui);
                } else {
                    self.transcribe_ready(ui);
                }
            });
        }
        ui.add_space(80.0);
    }

    fn transcribe_running(&mut self, ui: &mut egui::Ui) {
        sheet_title(ui, "Transcribing");
        ui.add_space(16.0);
        let message = if !self.state.transcribe_message.is_empty() {
            self.state.transcribe_message.clone()
        } else if self.state.transcribe_stage == PipelineStage::Transcribing {
            "Writing the words…".to_string()
        } else {
            "Reading the audio…".to_string()
        };
        ui.label(RichText::new(message).size(18.0).color(CREAM));
        ui.add_space(20.0);
        export_progress(ui, self.state.transcribe_percent);
        ui.add_space(24.0);
        if review_secondary(ui, "Cancel").clicked() {
            self.cancel();
        }
    }

    fn transcribe_failed(&mut self, ui: &mut egui::Ui) {
        let error = self.state.transcribe_error.clone().unwrap_or_default();
        match failure_kind(&error) {
            FailureKind::Model => self.missing_model(ui),
            FailureKind::Program => self.missing_program(ui),
            FailureKind::Other => self.transcription_failed(ui, &error),
        }
    }

    fn missing_model(&mut self, ui: &mut egui::Ui) {
        sheet_title(ui, "The speech model is missing");
        ui.add_space(12.0);
        ui.label(
            RichText::new("Transcription needs this file before it can write the words.")
                .size(18.0)
                .color(MUTED),
        );
        ui.add_space(28.0);
        micro(ui, "THE FILE");
        ui.add_space(8.0);
        file_path(ui, &sidecars::model_path().display().to_string());
        ui.add_space(24.0);
        if primary_lg(ui, "Try again").clicked() {
            self.start_transcribe();
        }
    }

    fn missing_program(&mut self, ui: &mut egui::Ui) {
        sheet_title(ui, "Whisper is not built yet");
        ui.add_space(12.0);
        ui.label(
            RichText::new("Run scripts/build-whisper.sh, then try again.")
                .size(18.0)
                .color(MUTED),
        );
        ui.add_space(24.0);
        if primary_lg(ui, "Try again").clicked() {
            self.start_transcribe();
        }
    }

    fn transcription_failed(&mut self, ui: &mut egui::Ui, error: &str) {
        sheet_title(ui, "Transcription failed");
        ui.add_space(16.0);
        if !error.is_empty() {
            failure_card(ui, error);
            ui.add_space(24.0);
        }
        if primary_lg(ui, "Try again").clicked() {
            self.start_transcribe();
        }
    }

    fn transcribe_ready(&mut self, ui: &mut egui::Ui) {
        sheet_title(ui, "Transcribe");
        ui.add_space(12.0);
        ui.label(
            RichText::new("The speech stays on this computer.")
                .size(18.0)
                .color(MUTED),
        );
        ui.add_space(28.0);
        micro(ui, "SPOKEN LANGUAGE");
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for (lang, label) in [
                (VideoLanguage::Auto, "Auto-detect"),
                (VideoLanguage::En, "English"),
                (VideoLanguage::Pt, "Portuguese"),
                (VideoLanguage::Es, "Spanish"),
            ] {
                if choice(ui, label, self.state.language == lang).clicked() {
                    self.dispatch(WizardAction::SetLanguage(lang));
                }
            }
        });
        ui.add_space(28.0);
        let saved = self.state.segments.len();
        if saved > 0 {
            self.saved_transcript(ui, saved);
        } else if primary_lg(ui, "Start transcribing").clicked() {
            self.start_transcribe();
        }
        ui.add_space(28.0);
        self.repair_settings(ui);
    }

    fn saved_transcript(&mut self, ui: &mut egui::Ui, saved: usize) {
        egui::Frame::new()
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, HAIRLINE))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(20.0)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "This video already has a transcript ({saved} {}).",
                        if saved == 1 { "line" } else { "lines" }
                    ))
                    .size(16.0)
                    .color(CREAM),
                );
                ui.add_space(14.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    if primary_lg(ui, "Use the saved transcript").clicked() {
                        self.dispatch(WizardAction::UseSavedTranscript);
                    }
                    if review_secondary(ui, "Transcribe again").clicked() {
                        self.start_transcribe();
                    }
                });
            });
    }

    fn repair_settings(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(
            RichText::new("If the words come out wrong")
                .size(18.0)
                .color(CREAM_HEAD),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("Leave these as they are unless a transcript is missing words, full of junk, or stuck repeating itself.")
                    .size(16.0)
                    .color(MUTED),
            );
            let mut entropy = self.state.entropy_thold as f32;
            if setting_slider(
                ui,
                "Drop uncertain words",
                &shown_number(entropy, 1),
                egui::Slider::new(&mut entropy, 0.0..=10.0).max_decimals(1),
            )
            .changed()
            {
                self.dispatch(WizardAction::SetEntropyThold(entropy as f64));
            }
            let mut context = self.state.max_context as f32;
            if setting_slider(
                ui,
                "Earlier words to remember",
                &shown_number(context, 0),
                egui::Slider::new(&mut context, 0.0..=128.0).max_decimals(0),
            )
            .changed()
            {
                self.dispatch(WizardAction::SetMaxContext(context as i64));
            }
            let mut beam = self.state.beam_size as f32;
            if setting_slider(
                ui,
                "Tries per line",
                &shown_number(beam, 0),
                egui::Slider::new(&mut beam, 1.0..=8.0).max_decimals(0),
            )
            .changed()
            {
                self.dispatch(WizardAction::SetBeamSize(beam as i64));
            }
            let mut temp = self.state.temperature_inc as f32;
            if setting_slider(
                ui,
                "Retry a stuck line",
                &shown_number(temp, 2),
                egui::Slider::new(&mut temp, 0.0..=1.0).max_decimals(2),
            )
            .changed()
            {
                self.dispatch(WizardAction::SetTemperatureInc(temp as f64));
            }
        });
    }

    pub(super) fn transcribe_done(&mut self, ui: &mut egui::Ui) {
        let lines = self.state.segments.len();
        let video = self.state.video_path.clone();
        let status = self.status.clone();
        let segments = self.state.segments.clone();
        with_sheet(ui, |ui| {
            sheet_title(ui, "Transcript saved");
            ui.add_space(12.0);
            ui.label(
                RichText::new(format!(
                    "{lines} {} saved in a text file in the same folder as the video.",
                    if lines == 1 { "line is" } else { "lines are" }
                ))
                .size(18.0)
                .color(MUTED),
            );
            if let Some(path) = &video {
                let text = sidecars::transcript_text_path(std::path::Path::new(path));
                ui.add_space(28.0);
                micro(ui, "THE FILE");
                ui.add_space(8.0);
                file_path(ui, &text.display().to_string());
                ui.add_space(24.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    if primary_lg(ui, "Open transcription").clicked() {
                        if let Err(error) = backend::open_path(&text) {
                            self.status = error;
                        }
                    }
                    if review_secondary(ui, "Back to home").clicked() {
                        self.go_home();
                    }
                });
            } else if primary_lg(ui, "Back to home").clicked() {
                self.go_home();
            }
            if !segments.is_empty() {
                ui.add_space(28.0);
                egui::CollapsingHeader::new(
                    RichText::new(format!(
                        "The words · {lines} {}",
                        if lines == 1 { "line" } else { "lines" }
                    ))
                    .size(18.0)
                    .color(CREAM_HEAD),
                )
                .default_open(true)
                .show(ui, |ui| {
                    for segment in &segments {
                        ui.label(
                            RichText::new(format!(
                                "{}   {}",
                                clock(segment.start_ms),
                                segment.text
                            ))
                            .size(15.0)
                            .color(CREAM),
                        );
                    }
                });
            }
            if !status.is_empty() {
                ui.add_space(16.0);
                ui.label(RichText::new(status).size(16.0).color(ORANGE));
            }
        });
        ui.add_space(80.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{failure_kind, FailureKind};

    #[test]
    fn a_missing_model_and_a_missing_program_are_their_own_screens() {
        assert_eq!(
            failure_kind("The Whisper model is not in ~/.a1slice/models yet."),
            FailureKind::Model
        );
        assert_eq!(
            failure_kind(
                "The whisper program is not built yet. Run scripts/build-whisper.sh, then try again."
            ),
            FailureKind::Program
        );
        assert_eq!(
            failure_kind("Whisper stopped: out of memory"),
            FailureKind::Other
        );
    }
}
