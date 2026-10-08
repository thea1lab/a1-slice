//! Export progress, the finished folder, and the failure sheet.

use crate::backend::{self};
use a1slice_core::types::*;
use eframe::egui::{self, RichText};

use super::support::file_name;
use super::theme::{CREAM, CREAM_HEAD, MUTED, ORANGE};
use super::widgets::{export_progress, primary_lg, review_secondary, secondary, with_sheet};
use super::A1App;

impl A1App {
    pub(super) fn export_screen(&mut self, ui: &mut egui::Ui) {
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
}
