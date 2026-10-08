//! Find best parts screen.

use a1slice_core::types::*;
use a1slice_core::wizard::{ReturnTo, WizardAction};
use eframe::egui::{self, Color32, RichText, Stroke};

use super::support::clock;
use super::theme::{CREAM, CREAM_HEAD, HAIRLINE, MUTED, SURFACE};
use super::widgets::{choice, danger, primary_lg, progress, secondary, with_sheet};
use super::A1App;

impl A1App {
    pub(super) fn find(&mut self, ui: &mut egui::Ui) {
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
}
