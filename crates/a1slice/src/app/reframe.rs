//! Reframe screen and the saved crop.

use crate::backend::{self};
use a1slice_core::crop::{self, CROP_PRESETS};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, RichText};

use super::support::current_crop;
use super::theme::{MUTED, ORANGE};
use super::widgets::{primary, progress};
use super::A1App;

impl A1App {
    pub(super) fn reframe(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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

    pub(super) fn store_crop(&mut self, crop: ClipCrop) {
        let mut framing = self.state.framing.clone();
        framing.insert(crop.ratio, crop);
        self.dispatch(WizardAction::SetFraming(framing.clone()));
        if let Some(path) = &self.state.video_path {
            let _ = sidecars::write_framing(std::path::Path::new(path), &framing);
        }
    }
}
