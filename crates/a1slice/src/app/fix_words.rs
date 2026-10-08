//! Fix the words: split long lines, then optionally ask an agent about typos.

use crate::backend::{self, JobDone};
use a1slice_core::caption_edit::{
    clamp_words_per_line, lines_longer_than, split_caption_note, CaptionEditRequest,
};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::support::active_cues;

use super::theme::{CREAM, CREAM_HEAD, MUTED, ORANGE};
use super::widgets::{choice, primary_lg, review_secondary, with_sheet};
use super::A1App;

impl A1App {
    pub(super) fn fix_words(&mut self, ui: &mut egui::Ui) {
        with_sheet(ui, |ui| {
            ui.label(
                RichText::new("Fix the words")
                    .font(egui::FontId::new(
                        40.0,
                        egui::FontFamily::Name("Noto Serif".into()),
                    ))
                    .color(CREAM_HEAD),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Long lines are split here. An agent can also correct typos and names.",
                )
                .size(16.0)
                .color(CREAM),
            );
            ui.add_space(16.0);
            self.fix_toggles(ui);
            ui.add_space(12.0);
            self.fix_agents(ui);
            ui.add_space(12.0);
            ui.label(RichText::new("Anything else").size(14.0).color(MUTED));
            ui.add_space(4.0);
            note_field(ui, &mut self.fix_request);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let running = self.job.is_some();
                if !running && primary_lg(ui, "Fix the words").clicked() {
                    self.ask_fix();
                }
                if running && review_secondary(ui, "Stop").clicked() {
                    self.cancel();
                }
            });
            if !self.fix_log.is_empty() {
                ui.add_space(12.0);
                ui.label(RichText::new(&self.fix_log).size(14.0).color(MUTED));
            }
            self.pending_fix(ui);
            ui.add_space(16.0);
            if review_secondary(ui, "Back to captions").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Captions));
            }
            ui.add_space(80.0);
        });
    }

    fn fix_toggles(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            if choice(ui, "Break long lines", self.fix_break_lines).clicked() {
                self.fix_break_lines = !self.fix_break_lines;
            }
            if choice(ui, "Fix typos", self.fix_typos).clicked() {
                self.fix_typos = !self.fix_typos;
            }
        });
        if !self.fix_break_lines {
            return;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new("Words on a line").size(14.0).color(MUTED));
            let response = number_field(ui, &mut self.fix_words_per_line);
            if response.changed() {
                self.fix_words_per_line = self
                    .fix_words_per_line
                    .chars()
                    .filter(|c| c.is_ascii_digit())
                    .take(2)
                    .collect();
            }
            if response.lost_focus() {
                self.fix_words_per_line = read_words_per_line(&self.fix_words_per_line).to_string();
            }
        });
    }

    fn fix_agents(&mut self, ui: &mut egui::Ui) {
        if self.agents.is_empty() {
            ui.label(
                RichText::new("This computer has no agent to run. You can still split the lines.")
                    .size(14.0)
                    .color(MUTED),
            );
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            for (id, label) in self.agents.clone() {
                if choice(ui, label, self.agent_id.as_deref() == Some(id)).clicked() {
                    self.agent_id = Some(id.to_string());
                }
            }
        });
    }

    pub(super) fn ask_fix(&mut self) {
        if self.job.is_some() {
            return;
        }
        let limit = read_words_per_line(&self.fix_words_per_line);
        self.fix_words_per_line = limit.to_string();
        let mut working = active_cues(&self.state, self.caption_source);
        let mut notes = Vec::new();
        if self.fix_break_lines {
            let (next, note) = split_caption_note(&working, limit as f64);
            if next != working {
                self.pending_lines = Some(next.clone());
            }
            notes.push(note);
            working = next;
        }
        self.fix_log = notes.join("\n");
        let note = self.fix_request.trim().to_string();
        let wants_agent = self.fix_typos || !note.is_empty();
        if !self.fix_break_lines && !wants_agent {
            self.fix_log = "Choose what to fix.".into();
            return;
        }
        if !wants_agent {
            return;
        }
        if working.is_empty() {
            self.push_fix_log("There are no words to fix yet.");
            return;
        }
        let Some(agent) = self.agent_id.clone() else {
            self.push_fix_log(
                "This computer has no agent to check typos. The lines are ready to save.",
            );
            return;
        };
        self.fix_basis = Some(working.clone());
        let request = CaptionEditRequest {
            fix_typos: self.fix_typos,
            break_lines: false,
            words_per_line: limit as f64,
            note,
        };
        self.push_fix_log(&format!("Starting {agent}."));
        self.job = Some(backend::spawn_job(move |cancel, child| {
            let lines = backend::fix_words(&working, &agent, &request, &cancel, &child)?;
            Ok(JobDone::CaptionLines(lines))
        }));
    }

    pub(super) fn take_fix_result(&mut self, lines: Vec<TranscriptSegment>) {
        let limit = read_words_per_line(&self.fix_words_per_line) as f64;
        if self.fix_break_lines && lines_longer_than(&lines, limit) > 0 {
            self.push_fix_log("The typo check tried to join lines. The shorter lines stayed.");
            return;
        }
        if self.fix_basis.as_ref().is_some_and(|basis| basis == &lines) {
            self.push_fix_log("No typos to fix.");
            return;
        }
        self.pending_lines = Some(lines);
        self.push_fix_log("The edit is ready. Save the lines to keep them.");
    }

    pub(super) fn push_fix_log(&mut self, line: &str) {
        if self.fix_log.is_empty() {
            self.fix_log = line.to_string();
        } else {
            self.fix_log.push('\n');
            self.fix_log.push_str(line);
        }
    }

    fn pending_fix(&mut self, ui: &mut egui::Ui) {
        let Some(lines) = self.pending_lines.clone() else {
            return;
        };
        ui.add_space(16.0);
        ui.label(RichText::new("New words").size(18.0).color(CREAM_HEAD));
        ui.add_space(8.0);
        for line in &lines {
            ui.label(&line.text);
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if primary_lg(ui, "Save the lines").clicked() {
                self.dispatch(WizardAction::SetSegments(lines.clone()));
                self.remember_captions();
                self.pending_lines = None;
                self.fix_basis = None;
                self.dispatch(WizardAction::ShowScreen(Screen::Captions));
            }
            if review_secondary(ui, "Reject").clicked() {
                self.pending_lines = None;
            }
        });
    }
}

pub(super) fn read_words_per_line(value: &str) -> i64 {
    if value.trim().is_empty() {
        8
    } else {
        clamp_words_per_line(value.parse::<f64>().unwrap_or(8.0))
    }
}

fn number_field(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    field(ui, |ui| {
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(64.0)
                .min_size(egui::vec2(64.0, 44.0))
                .margin(egui::Margin::symmetric(8, 0))
                .horizontal_align(egui::Align::Center)
                .vertical_align(egui::Align::Center)
                .font(egui::FontId::proportional(16.0)),
        )
    })
}

fn note_field(ui: &mut egui::Ui, text: &mut String) {
    field(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(text)
                .desired_width(f32::INFINITY)
                .desired_rows(2)
                .hint_text("For example: the name is Anna, not Ana")
                .font(egui::FontId::proportional(16.0)),
        )
    });
}

fn field(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> egui::Response) -> egui::Response {
    ui.scope(|ui| {
        {
            let look = ui.style_mut();
            let input = Color32::from_rgb(0x14, 0x14, 0x14);
            let edge = Color32::from_rgb(0x3a, 0x3a, 0x3a);
            for visuals in [
                &mut look.visuals.widgets.inactive,
                &mut look.visuals.widgets.hovered,
                &mut look.visuals.widgets.active,
            ] {
                visuals.corner_radius = egui::CornerRadius::same(8);
                visuals.bg_fill = input;
            }
            look.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, edge);
            look.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ORANGE);
            look.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ORANGE);
            look.visuals.extreme_bg_color = input;
            look.visuals.override_text_color = Some(CREAM);
        }
        add(ui)
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::read_words_per_line;

    #[test]
    fn an_empty_word_count_is_eight_and_the_ends_clamp() {
        assert_eq!(read_words_per_line(""), 8);
        assert_eq!(read_words_per_line("1"), 2);
        assert_eq!(read_words_per_line("8"), 8);
        assert_eq!(read_words_per_line("40"), 24);
    }
}
