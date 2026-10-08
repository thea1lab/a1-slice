//! Captions screen and Fix the words.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, RichText};

use super::caption_controls::{color_row, row_label, style_row};
use super::layout::{picture_room, viewport_height};
use super::support::{active_cues, caption_project};
use super::theme::{CREAM_HEAD, MUTED, ORANGE};
use super::widgets::{choice, export_progress, primary_lg, review_secondary, secondary};
use super::A1App;

const CLUSTER_W: f32 = 1000.0;

fn wide_dock(width: f32) -> bool {
    width >= CLUSTER_W + 16.0
}

fn cluster_width(width: f32) -> f32 {
    if wide_dock(width) {
        CLUSTER_W
    } else {
        width
    }
}

/// First-frame guess. The next frame uses the height the dock actually took.
fn caption_dock_estimate(wide: bool) -> f32 {
    if wide {
        360.0
    } else {
        560.0
    }
}

fn export_action_label(look: CaptionLook) -> &'static str {
    if look == CaptionLook::Burn {
        "Export the video"
    } else {
        "Save the .srt"
    }
}

/// Choices stay on the left. The export action stays on the right, with a gap between them.
fn place_export_actions(
    ui: &mut egui::Ui,
    choices: impl FnOnce(&mut egui::Ui),
    action: impl FnOnce(&mut egui::Ui),
) {
    ui.scope(|ui| {
        ui.set_min_width(ui.available_width());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            action(ui);
            ui.add_space(28.0);
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    choices(ui);
                },
            );
        });
    });
}

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
        let avail_w = ui.available_width();
        let wide = wide_dock(avail_w);
        let reserve = if self.caption_dock_px > 1.0 {
            self.caption_dock_px + 4.0
        } else {
            caption_dock_estimate(wide)
        };
        let room = picture_room(viewport_height(ui), reserve, 180.0);
        self.picture_limited(ui, ctx, avail_w, room);
        let dock_top = ui.cursor().min.y;
        ui.add_space(8.0);
        let cluster = cluster_width(avail_w);
        let side = ((avail_w - cluster) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(cluster);
                ui.spacing_mut().item_spacing.y = 0.0;
                self.caption_controls(ui, wide);
            });
        });
        ui.add_space(4.0);
        self.caption_dock_px = (ui.cursor().min.y - dock_top).max(0.0);
    }

    fn caption_controls(&mut self, ui: &mut egui::Ui, wide: bool) {
        let mut changed = false;
        ui.label(RichText::new("Set the words").size(18.0).color(CREAM_HEAD));
        ui.add_space(4.0);
        let help = if self.caption_look == CaptionLook::Burn {
            "The picture shows the words where the exported video will put them."
        } else {
            "A sidecar leaves the picture as it is and writes an .srt next to the video."
        };
        ui.label(RichText::new(help).size(14.0).color(MUTED));
        ui.add_space(12.0);

        changed |= color_row(ui, &mut self.caption_style);
        ui.add_space(8.0);
        changed |= style_row(ui, &mut self.caption_style, wide);
        ui.add_space(8.0);
        changed |= self.words_row(ui);
        ui.add_space(8.0);
        self.export_row(ui);
        if changed {
            self.remember_captions();
        }
    }

    fn words_row(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            row_label(ui, "Words");
            if choice(
                ui,
                "Transcript",
                self.caption_source == CaptionSource::Transcript,
            )
            .clicked()
            {
                self.caption_source = CaptionSource::Transcript;
                changed = true;
            }
            if choice(
                ui,
                "Caption file",
                self.caption_source == CaptionSource::Manual,
            )
            .clicked()
            {
                self.caption_source = CaptionSource::Manual;
                changed = true;
            }
            if secondary(ui, "Choose a caption file").clicked() {
                changed |= self.pick_caption_file();
            }
        });
        ui.add_space(6.0);
        ui.label(RichText::new(self.words_path()).size(13.0).color(MUTED));
        if active_cues(&self.state, self.caption_source).is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "This video has no words yet. Transcribe it, or choose a caption file.",
                )
                .size(14.0)
                .color(MUTED),
            );
        }
        changed
    }

    fn words_path(&self) -> String {
        if self.caption_source == CaptionSource::Manual {
            return self
                .state
                .captions
                .as_ref()
                .and_then(|project| project.file_path.clone())
                .filter(|path| !path.trim().is_empty())
                .unwrap_or_else(|| "No caption file yet".into());
        }
        let Some(video) = self.state.video_path.as_deref() else {
            return "No transcript yet".into();
        };
        let path = sidecars::transcript_text_path(std::path::Path::new(video));
        if path.is_file() {
            path.display().to_string()
        } else {
            "No transcript yet".into()
        }
    }

    fn pick_caption_file(&mut self) -> bool {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Captions", &["srt", "txt", "vtt"])
            .pick_file()
        else {
            return false;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let cues = a1slice_core::project::parse_caption_document(&text);
                if cues.is_empty() {
                    self.status = "That file did not contain any cues.".into();
                    false
                } else {
                    self.caption_source = CaptionSource::Manual;
                    self.dispatch(WizardAction::SetSegments(cues.clone()));
                    self.status.clear();
                    self.save_captions(caption_project(
                        self.caption_source,
                        self.caption_look,
                        self.caption_style.clone(),
                        cues,
                        Some(path.display().to_string()),
                    ));
                    true
                }
            }
            Err(error) => {
                self.status = error.to_string();
                false
            }
        }
    }

    fn export_row(&mut self, ui: &mut egui::Ui) {
        let busy = self.state.export_stage == PipelineStage::Cutting;
        let look = self.caption_look;
        let mut next_look = None;
        let mut open_fix = false;
        let mut cancel = false;
        let mut export = false;
        place_export_actions(
            ui,
            |ui| {
                row_label(ui, "Export");
                if choice(ui, "Burn into the picture", look == CaptionLook::Burn).clicked() {
                    next_look = Some(CaptionLook::Burn);
                }
                if choice(ui, "Sidecar .srt", look == CaptionLook::Srt).clicked() {
                    next_look = Some(CaptionLook::Srt);
                }
                if !busy && review_secondary(ui, "Fix the words").clicked() {
                    open_fix = true;
                }
            },
            |ui| {
                if busy {
                    cancel = review_secondary(ui, "Cancel").clicked();
                } else {
                    export = primary_lg(ui, export_action_label(look)).clicked();
                }
            },
        );
        if let Some(look) = next_look {
            self.caption_look = look;
            self.remember_captions();
        }
        if open_fix {
            self.dispatch(WizardAction::ShowScreen(Screen::FixWords));
        }
        if cancel {
            self.cancel();
        }
        if export {
            self.export_captions_now();
        }
        if busy {
            ui.add_space(8.0);
            if !self.state.export_message.is_empty() {
                ui.label(
                    RichText::new(&self.state.export_message)
                        .size(14.0)
                        .color(MUTED),
                );
                ui.add_space(6.0);
            }
            export_progress(ui, self.state.export_percent);
        }
        if self.state.export_stage == PipelineStage::Done {
            if let Some(dir) = self.state.output_dir.clone() {
                ui.add_space(8.0);
                ui.label(RichText::new(&dir).size(13.0).color(CREAM_HEAD));
                ui.add_space(8.0);
                let label = if dir.ends_with(".srt") {
                    "Open the file"
                } else {
                    "Open folder"
                };
                if review_secondary(ui, label).clicked() {
                    let _ = backend::open_path(std::path::Path::new(&dir));
                }
            }
        }
        if !self.status.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(&self.status).color(ORANGE));
        }
        if let Some(error) = &self.state.export_error {
            ui.add_space(8.0);
            ui.label(RichText::new(error).color(ORANGE));
        }
    }

    fn export_captions_now(&mut self) {
        let cues = active_cues(&self.state, self.caption_source);
        if cues.is_empty() {
            self.status = "There are no words to export yet.".into();
            return;
        }
        self.status.clear();
        self.remember_captions();
        self.start_captions();
    }

    pub(super) fn remember_captions(&mut self) {
        let cues = active_cues(&self.state, self.caption_source);
        let file_path = if self.caption_source == CaptionSource::Manual {
            self.state
                .captions
                .as_ref()
                .and_then(|project| project.file_path.clone())
        } else {
            None
        };
        self.save_captions(caption_project(
            self.caption_source,
            self.caption_look,
            self.caption_style.clone(),
            cues,
            file_path,
        ));
    }

    pub(super) fn save_captions(&mut self, project: CaptionProject) {
        self.dispatch(WizardAction::SetCaptions(project.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_captions(std::path::Path::new(path), &project);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::widgets::{choice, primary_lg};
    use super::{
        caption_dock_estimate, cluster_width, export_action_label, picture_room,
        place_export_actions, wide_dock,
    };
    use a1slice_core::types::CaptionLook;

    #[test]
    fn caption_controls_fit_the_opening_window() {
        let window_h = 960.0;
        let header = 48.0;
        let dock = caption_dock_estimate(true);
        let picture = picture_room(window_h - header, dock, 180.0);
        assert!(picture >= 400.0, "the picture should lead, got {picture}");
        assert!(picture + dock + header <= window_h + 0.5);
        assert!(wide_dock(1280.0));
        assert!(!wide_dock(800.0));
        assert_eq!(cluster_width(1600.0), cluster_width(1280.0));
        assert!(cluster_width(1600.0) < 1100.0);
        let narrow = caption_dock_estimate(false);
        let narrow_picture = picture_room(window_h - header, narrow, 180.0);
        assert!(narrow_picture >= 180.0);
        assert!(narrow_picture + narrow + header <= window_h + 0.5);
    }

    #[test]
    fn export_names_the_video_and_sits_apart_from_the_choices() {
        assert_eq!(export_action_label(CaptionLook::Burn), "Export the video");
        assert_eq!(export_action_label(CaptionLook::Srt), "Save the .srt");
        let ctx = egui::Context::default();
        let mut action_right = 0.0_f32;
        let mut choice_right = 0.0_f32;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0, 800.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.set_max_width(1000.0);
                    place_export_actions(
                        ui,
                        |ui| {
                            let response = choice(ui, "Burn into the picture", true);
                            choice_right = response.rect.right();
                        },
                        |ui| {
                            let response = primary_lg(ui, "Export the video");
                            action_right = response.rect.right();
                        },
                    );
                });
            },
        );
        assert!(
            action_right > choice_right + 24.0,
            "the export action should sit to the right of the choices, action {action_right} choice {choice_right}"
        );
    }
}
