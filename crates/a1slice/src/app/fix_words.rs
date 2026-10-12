//! Fix the words: split long lines, then optionally ask an agent about typos.

use crate::backend::{self, JobDone};
use a1slice_core::caption_edit::{
    clamp_words_per_line, lines_longer_than, split_caption_note, CaptionEditRequest,
};
use a1slice_core::sidecars;
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::support::{active_cues, caption_project, clock};

use super::theme::{CREAM, CREAM_HEAD, HAIRLINE, MUTED, ORANGE, STONE, SURFACE};
use super::widgets::{choice, primary_lg, review_secondary};
use super::A1App;

const SAVE_CAPTIONS: &str = "Save the new captions";
const COLUMN_GAP: f32 = 40.0;
const TIME_W: f32 = 76.0;

impl A1App {
    pub(super) fn fix_words(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        let open = self.proposal_open();
        let bounds = egui::Rect::from_min_size(ui.cursor().min, ui.available_size());
        let frames = fix_frames(bounds, open);
        if let Some(right) = frames.right {
            self.fix_columns(ui, frames.left, right);
        } else {
            self.fix_stack(ui, frames.left, open);
        }
        ui.add_space(24.0);
    }

    fn proposal_open(&self) -> bool {
        !self.fix_dismissed && (self.pending_lines.is_some() || self.job.is_some())
    }

    fn fix_columns(&mut self, ui: &mut egui::Ui, left: egui::Rect, right: egui::Rect) {
        let view_h = (ui.available_height() - 16.0).max(420.0);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let shift = (left.left() - ui.cursor().min.x).max(0.0);
            ui.add_space(shift);
            let column_top = ui.cursor().min.y;
            ui.vertical(|ui| {
                ui.set_width(left.width());
                self.fix_form(ui);
                ui.add_space(20.0);
                let used = ui.cursor().min.y - column_top;
                let list_h = (view_h - used).max(200.0);
                self.show_current(ui, list_h);
            });
            ui.add_space((right.left() - left.right()).max(0.0));
            ui.vertical(|ui| {
                ui.set_width(right.width());
                self.proposal_column(ui, view_h);
            });
        });
    }

    fn fix_stack(&mut self, ui: &mut egui::Ui, left: egui::Rect, open: bool) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let shift = (left.left() - ui.cursor().min.x).max(0.0);
            ui.add_space(shift);
            ui.vertical(|ui| {
                ui.set_width(left.width());
                self.fix_form(ui);
                ui.add_space(20.0);
                let room = ui.available_height();
                let list_h = if room.is_finite() {
                    room.clamp(240.0, if open { 320.0 } else { 480.0 })
                } else {
                    360.0
                };
                self.show_current(ui, list_h);
                if open {
                    ui.add_space(20.0);
                    self.proposal_column(ui, 420.0);
                }
            });
        });
    }

    fn fix_form(&mut self, ui: &mut egui::Ui) {
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
            RichText::new("Long lines are split here. An agent can also correct typos and names.")
                .size(16.0)
                .color(CREAM),
        );
        ui.add_space(16.0);
        self.fix_toggles(ui);
        ui.add_space(16.0);
        self.fix_agents(ui);
        ui.add_space(16.0);
        ui.label(RichText::new("Anything else").size(14.0).color(MUTED));
        ui.add_space(8.0);
        note_field(ui, &mut self.fix_request);
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            let running = self.job.is_some();
            if !running && primary_lg(ui, "Fix the caption").clicked() {
                self.ask_fix();
            }
            if running && review_secondary(ui, "Stop").clicked() {
                self.cancel();
            }
            if review_secondary(ui, "Back to captions").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Captions));
            }
        });
        if !self.fix_log.is_empty() {
            ui.add_space(12.0);
            ui.label(RichText::new(&self.fix_log).size(14.0).color(MUTED));
        }
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
        let (response, _) = placed_words_per_line(ui, &mut self.fix_words_per_line);
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
        self.fix_dismissed = false;
        self.fix_drop_result = false;
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
        if self.fix_drop_result {
            self.fix_drop_result = false;
            return;
        }
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
        self.fix_dismissed = false;
        self.push_fix_log("The edit is ready. Save the new captions to keep them.");
    }

    pub(super) fn push_fix_log(&mut self, line: &str) {
        if self.fix_log.is_empty() {
            self.fix_log = line.to_string();
        } else {
            self.fix_log.push('\n');
            self.fix_log.push_str(line);
        }
    }

    fn show_current(&mut self, ui: &mut egui::Ui, list_h: f32) {
        ui.label(RichText::new("Captions").size(18.0).color(CREAM_HEAD));
        ui.add_space(8.0);
        let lines = active_cues(&self.state, self.caption_source);
        caption_scroll(ui, "fix-current", &lines, list_h);
    }

    fn proposal_column(&mut self, ui: &mut egui::Ui, height: f32) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), height),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(12), SURFACE);
        let inner = rect.shrink2(egui::vec2(16.0, 16.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
            self.proposal_body(ui, inner.height());
        });
    }

    fn proposal_body(&mut self, ui: &mut egui::Ui, height: f32) {
        let top = ui.max_rect().top();
        ui.set_width(ui.available_width());
        ui.label(RichText::new("New captions").size(18.0).color(CREAM_HEAD));
        ui.add_space(8.0);
        let running = self.job.is_some();
        if running {
            ui.label(
                RichText::new(self.fix_running_line())
                    .size(14.0)
                    .color(MUTED),
            );
            ui.add_space(8.0);
        }
        let lines = self.pending_lines.clone().unwrap_or_default();
        let footer = 64.0;
        let list_h = (top + height - ui.cursor().min.y - footer).max(80.0);
        if lines.is_empty() {
            if !running {
                ui.label(
                    RichText::new("The new captions will show up here.")
                        .size(15.0)
                        .color(MUTED),
                );
            }
        } else {
            caption_scroll(ui, "fix-new", &lines, list_h);
        }
        ui.add_space(16.0);
        let (save, reject) = proposal_actions(ui, !running && !lines.is_empty());
        if reject {
            self.reject_fix();
        } else if save {
            self.keep_new_captions(lines);
        }
    }

    fn fix_running_line(&self) -> String {
        let name = self
            .agents
            .iter()
            .find(|(id, _)| self.agent_id.as_deref() == Some(*id))
            .map(|(_, label)| *label)
            .unwrap_or("The agent");
        if self.pending_lines.is_some() {
            format!("{name} is still checking these lines.")
        } else {
            format!("{name} is reading the captions.")
        }
    }

    fn reject_fix(&mut self) {
        self.pending_lines = None;
        self.fix_basis = None;
        self.fix_dismissed = true;
        if self.job.is_some() {
            self.fix_drop_result = true;
            self.cancel();
        }
    }

    fn keep_new_captions(&mut self, lines: Vec<TranscriptSegment>) {
        if self.job.is_some() || lines.is_empty() {
            return;
        }
        if self.caption_source == CaptionSource::Manual {
            let file_path = self
                .state
                .captions
                .as_ref()
                .and_then(|project| project.file_path.clone());
            self.save_captions(caption_project(
                self.caption_source,
                self.caption_look,
                self.caption_style.clone(),
                lines,
                file_path,
            ));
        } else {
            self.dispatch(WizardAction::SetSegments(lines.clone()));
            self.remember_captions();
            if let Some(path) = &self.state.video_path {
                let _ = sidecars::write_transcript(std::path::Path::new(path), &lines);
            }
        }
        self.pending_lines = None;
        self.fix_basis = None;
        self.fix_dismissed = false;
        self.dispatch(WizardAction::ShowScreen(Screen::Captions));
    }
}

struct FixFrames {
    left: egui::Rect,
    right: Option<egui::Rect>,
}

/// Wide windows put the proposal beside the form. Narrow windows keep one column.
fn fix_frames(bounds: egui::Rect, open: bool) -> FixFrames {
    let inset = 20.0;
    let usable = (bounds.width() - inset * 2.0).max(160.0);
    if open && usable >= 960.0 {
        let col = ((usable - COLUMN_GAP) / 2.0).min(640.0);
        let pair = col * 2.0 + COLUMN_GAP;
        let x = bounds.left() + inset + ((usable - pair) * 0.5).max(0.0);
        let left = egui::Rect::from_min_size(
            egui::pos2(x, bounds.top()),
            egui::vec2(col, bounds.height()),
        );
        let right = egui::Rect::from_min_size(
            egui::pos2(left.right() + COLUMN_GAP, bounds.top()),
            egui::vec2(col, bounds.height()),
        );
        FixFrames {
            left,
            right: Some(right),
        }
    } else {
        let width = usable.min(760.0);
        let x = bounds.left() + inset + ((usable - width) * 0.5).max(0.0);
        FixFrames {
            left: egui::Rect::from_min_size(
                egui::pos2(x, bounds.top()),
                egui::vec2(width, bounds.height()),
            ),
            right: None,
        }
    }
}

fn caption_scroll(ui: &mut egui::Ui, salt: &str, lines: &[TranscriptSegment], height: f32) {
    egui::ScrollArea::vertical()
        .id_salt(salt)
        .auto_shrink([false, false])
        .max_height(height)
        .show(ui, |ui| {
            ui.set_width((ui.available_width() - 8.0).max(80.0));
            cue_rows(ui, lines);
        });
}

fn cue_rows(ui: &mut egui::Ui, lines: &[TranscriptSegment]) {
    if lines.is_empty() {
        ui.label(
            RichText::new("This video has no words yet. Transcribe it, or choose a caption file.")
                .size(15.0)
                .color(MUTED),
        );
        return;
    }
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            ui.add_space(8.0);
        }
        let row = cue_row(ui, line);
        ui.add_space(8.0);
        ui.painter().hline(
            row.x_range(),
            ui.cursor().min.y,
            egui::Stroke::new(1.0_f32, HAIRLINE),
        );
        ui.add_space(8.0);
    }
}

fn cue_row(ui: &mut egui::Ui, line: &TranscriptSegment) -> egui::Rect {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.allocate_ui_with_layout(
            egui::vec2(TIME_W, 0.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.label(RichText::new(clock(line.start_ms)).size(14.0).color(MUTED));
            },
        );
        ui.add(
            egui::Label::new(RichText::new(line.text.trim()).size(16.0).color(CREAM))
                .wrap_mode(egui::TextWrapMode::Wrap),
        );
    })
    .response
    .rect
}

fn proposal_actions(ui: &mut egui::Ui, can_save: bool) -> (bool, bool) {
    let mut save = false;
    let mut reject = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        if can_save && primary_lg(ui, SAVE_CAPTIONS).clicked() {
            save = true;
        }
        if review_secondary(ui, "Reject").clicked() {
            reject = true;
        }
    });
    (save, reject)
}

pub(super) fn read_words_per_line(value: &str) -> i64 {
    if value.trim().is_empty() {
        8
    } else {
        clamp_words_per_line(value.parse::<f64>().unwrap_or(8.0))
    }
}

fn placed_words_per_line(ui: &mut egui::Ui, text: &mut String) -> (egui::Response, egui::Rect) {
    let label = ui.label(RichText::new("Words on a line").size(14.0).color(MUTED));
    ui.add_space(4.0);
    (number_field(ui, text), label.rect)
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
                .hint_text(RichText::new("For example: the name is Anna, not Ana").color(STONE))
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
    use super::{
        cue_row, cue_rows, fix_frames, note_field, placed_words_per_line, proposal_actions,
        read_words_per_line, SAVE_CAPTIONS,
    };
    use crate::app::theme::STONE;
    use a1slice_core::types::TranscriptSegment;

    #[test]
    fn an_empty_word_count_is_eight_and_the_ends_clamp() {
        assert_eq!(read_words_per_line(""), 8);
        assert_eq!(read_words_per_line("1"), 2);
        assert_eq!(read_words_per_line("8"), 8);
        assert_eq!(read_words_per_line("40"), 24);
    }

    #[test]
    fn the_word_count_sits_under_its_label() {
        let ctx = egui::Context::default();
        let mut text = "8".to_string();
        let mut label = egui::Rect::NOTHING;
        let mut field = egui::Rect::NOTHING;
        let _ = ctx.run(screen(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (response, label_rect) = placed_words_per_line(ui, &mut text);
                field = response.rect;
                label = label_rect;
            });
        });
        assert!(
            field.top() >= label.bottom() - 1.0,
            "the number sits under the label, label {} field {}",
            label.bottom(),
            field.top()
        );
        assert!(
            (field.left() - label.left()).abs() < 1.0,
            "the number lines up with the label, label {} field {}",
            label.left(),
            field.left()
        );
    }

    #[test]
    fn the_note_placeholder_is_gray() {
        let ctx = egui::Context::default();
        let mut text = String::new();
        let output = ctx.run(screen(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                note_field(ui, &mut text);
            });
        });
        let mut sections = Vec::new();
        for clipped in output.shapes {
            hint_sections(&clipped.shape, &mut sections);
        }
        let placeholder = sections
            .iter()
            .find(|(text, _)| text.contains("Anna"))
            .map(|(_, color)| *color);
        assert_eq!(
            placeholder,
            Some(STONE),
            "placeholder should be stone gray, got {sections:?}"
        );
    }

    fn hint_sections(shape: &egui::Shape, sections: &mut Vec<(String, egui::Color32)>) {
        match shape {
            egui::Shape::Text(text) => {
                let job = &text.galley.job;
                for section in &job.sections {
                    let slice = job.text[section.byte_range.clone()].to_string();
                    sections.push((slice, section.format.color));
                }
            }
            egui::Shape::Vec(children) => {
                for child in children {
                    hint_sections(child, sections);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn a_wide_proposal_opens_on_the_right() {
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
        let frames = fix_frames(bounds, true);
        let right = frames.right.expect("proposal");
        assert!(
            right.left() >= frames.left.right() + 32.0,
            "the new captions sit to the right, left {} right {}",
            frames.left.right(),
            right.left()
        );
        assert!((right.top() - frames.left.top()).abs() < 1.0);
        assert!(right.right() <= bounds.right() - 19.0);
    }

    #[test]
    fn a_narrow_window_keeps_one_column() {
        let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0));
        let frames = fix_frames(bounds, true);
        assert!(frames.right.is_none());
    }

    #[test]
    fn a_caption_row_shows_its_time_on_the_left() {
        let ctx = egui::Context::default();
        let output = ctx.run(screen(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                cue_row(
                    ui,
                    &TranscriptSegment {
                        start_ms: 65_000,
                        end_ms: 68_000,
                        text: "Hello there".into(),
                    },
                );
            });
        });
        let mut time_x = None;
        let mut text_x = None;
        let mut texts = Vec::new();
        for clipped in output.shapes {
            text_at(&clipped.shape, &mut texts);
        }
        for (text, pos) in &texts {
            if text.contains("1:05") {
                time_x = Some(pos.x);
            }
            if text.contains("Hello there") {
                text_x = Some(pos.x);
            }
        }
        let (time_x, text_x) = (time_x.expect("time"), text_x.expect("words"));
        assert!(
            text_x > time_x + 40.0,
            "the words sit to the right of the time, time {time_x} words {text_x}"
        );
    }

    #[test]
    fn a_long_caption_list_scrolls() {
        let lines: Vec<_> = (0..40)
            .map(|index| TranscriptSegment {
                start_ms: index * 2_000,
                end_ms: index * 2_000 + 1_500,
                text: format!("Line {index} of the caption"),
            })
            .collect();
        let ctx = egui::Context::default();
        let mut content_h = 0.0;
        let mut view_h = 0.0;
        let _ = ctx.run(screen(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let shown = egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| cue_rows(ui, &lines));
                content_h = shown.content_size.y;
                view_h = shown.inner_rect.height();
            });
        });
        assert!(
            content_h > view_h + 20.0,
            "a long transcript scrolls inside the list, content {content_h} view {view_h}"
        );
    }

    #[test]
    fn the_proposal_offers_save_and_reject() {
        let ctx = egui::Context::default();
        let output = ctx.run(screen(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                proposal_actions(ui, true);
            });
        });
        let mut save_x = None;
        let mut reject_x = None;
        let mut texts = Vec::new();
        for clipped in output.shapes {
            text_at(&clipped.shape, &mut texts);
        }
        for (text, pos) in &texts {
            if text.contains(SAVE_CAPTIONS) {
                save_x = Some(pos.x);
            }
            if text.contains("Reject") {
                reject_x = Some(pos.x);
            }
        }
        let (save_x, reject_x) = (save_x.expect("save"), reject_x.expect("reject"));
        assert!(save_x < reject_x, "save sits before reject");
    }

    fn text_at(shape: &egui::Shape, texts: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => {
                let job = &text.galley.job;
                for section in &job.sections {
                    let slice = job.text[section.byte_range.clone()].to_string();
                    texts.push((slice, text.pos));
                }
            }
            egui::Shape::Vec(children) => {
                for child in children {
                    text_at(child, texts);
                }
            }
            _ => {}
        }
    }

    fn screen() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 400.0),
            )),
            ..Default::default()
        }
    }
}
