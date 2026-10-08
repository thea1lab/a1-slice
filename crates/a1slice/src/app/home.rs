//! Home cards and the tool intro shared by the other screens.

use a1slice_core::types::*;
use eframe::egui::{self, Color32, RichText, Stroke};
use std::path::PathBuf;

use super::paint::paint_tool_icon;
use super::support::file_name;
use super::theme::{BEIGE, CREAM, CREAM_HEAD, MUTED, ORANGE};
use super::widgets::{home_tile, primary_lg, recent_row};
use super::A1App;

impl A1App {
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        // Stay inside the visible panel. A centered line wider than the window
        // otherwise pulls the cards past the left clip edge.
        let bounds = ui.max_rect();
        let inset = 20.0;
        let width = (bounds.width() - inset * 2.0).max(160.0);
        let narrow = width < 720.0;
        let grid_w = width.min(920.0);
        let gap = 16.0;
        let cols: usize = if narrow { 1 } else { 2 };
        let tile_w = (grid_w - gap * (cols as f32 - 1.0)) / cols as f32;
        let tile_h = if narrow {
            let screen_h = ui.ctx().screen_rect().height();
            let chrome = 44.0;
            let header = 128.0;
            ((screen_h - chrome - header - gap * 3.0) / 4.0).clamp(136.0, 176.0)
        } else {
            210.0
        };
        let headline = if width < 760.0 {
            40.0
        } else if width < 1100.0 {
            48.0
        } else {
            56.0
        };
        let serif = egui::FontId::new(headline, egui::FontFamily::Name("Noto Serif".into()));

        ui.add_space(if narrow { 4.0 } else { 36.0 });
        ui.vertical_centered(|ui| {
            ui.set_max_width(width);
            ui.label(
                RichText::new("A1 SLICE")
                    .size(11.0)
                    .color(MUTED)
                    .strong()
                    .extra_letter_spacing(1.0),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("What do you want to do?")
                    .font(serif)
                    .color(CREAM_HEAD),
            );
            ui.add_space(12.0);
            ui.label(
                RichText::new("Pick a tool. You’ll choose the video on the next screen.")
                    .size(16.0)
                    .color(MUTED),
            );
        });
        ui.add_space(if narrow { 16.0 } else { 40.0 });

        let tools = [
            (
                ToolId::Transcribe,
                "Transcribe",
                "Turn speech into a saved transcript.",
            ),
            (
                ToolId::Find,
                "Find best parts",
                "Keep the moments you want, then export them.",
            ),
            (
                ToolId::Reframe,
                "Reframe",
                "Fit the picture to Story, YouTube, or square.",
            ),
            (ToolId::Captions, "Captions", "Put the words on the video."),
        ];
        let rows = tools.len().div_ceil(cols);
        let grid_h = rows as f32 * tile_h + (rows.saturating_sub(1) as f32) * gap;
        let origin_y = ui.cursor().min.y;
        let left = bounds.left() + inset + ((width - grid_w) * 0.5).max(0.0);
        let _ = ui.allocate_exact_size(egui::vec2(bounds.width(), grid_h), egui::Sense::hover());
        let mut open = None;
        for (index, (id, title, detail)) in tools.into_iter().enumerate() {
            let col = index % cols;
            let row = index / cols;
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    left + col as f32 * (tile_w + gap),
                    origin_y + row as f32 * (tile_h + gap),
                ),
                egui::vec2(tile_w, tile_h),
            );
            if home_tile(ui, rect, title, detail, id, narrow).clicked() {
                open = Some(id);
            }
        }
        if let Some(id) = open {
            self.enter(id);
        }
        ui.add_space(80.0);
    }

    pub(super) fn tool_intro(
        &mut self,
        ui: &mut egui::Ui,
        tool: ToolId,
        title: &str,
        lead: &str,
        steps: &[&str],
    ) {
        let avail = ui.available_width();
        let sheet_w = avail.min(720.0).max(240.0);
        let side = ((avail - sheet_w) * 0.5).max(0.0);
        let title_size = if sheet_w < 520.0 { 40.0 } else { 52.0 };
        let serif = egui::FontId::new(title_size, egui::FontFamily::Name("Noto Serif".into()));
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(side);
            ui.vertical(|ui| {
                ui.set_width(sheet_w);
                let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::hover());
                ui.painter().rect(
                    icon_rect,
                    egui::CornerRadius::same(12),
                    CREAM_HEAD,
                    Stroke::new(1.0_f32, BEIGE),
                    egui::StrokeKind::Inside,
                );
                paint_tool_icon(ui.painter(), icon_rect, tool);
                ui.add_space(28.0);
                ui.label(RichText::new(title).font(serif).color(CREAM_HEAD));
                ui.add_space(12.0);
                ui.label(RichText::new(lead).size(18.0).color(MUTED));
                ui.add_space(28.0);
                ui.label(RichText::new("What you do").size(18.0).color(CREAM_HEAD));
                ui.add_space(12.0);
                for (index, step) in steps.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new(format!("{:02}", index + 1)).size(15.0).color(ORANGE).strong());
                        ui.add_space(8.0);
                        ui.label(RichText::new(*step).size(18.0).color(CREAM));
                    });
                    ui.add_space(10.0);
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    if primary_lg(ui, "Choose a video").clicked() {
                        self.choose_video(tool);
                    }
                    ui.label(RichText::new("MP4, MOV, MKV, AVI, or WebM").size(15.0).color(Color32::from_rgb(0x8a, 0x8a, 0x8a)));
                });
                if !self.recent.is_empty() {
                    ui.add_space(28.0);
                    ui.label(RichText::new("Recent videos").size(18.0).color(CREAM_HEAD));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Open a file you used before. To work on a different video, choose one above.").size(16.0).color(MUTED));
                    ui.add_space(10.0);
                    let picks = self.recent.clone();
                    let last = picks.len().saturating_sub(1);
                    for (index, (path, note)) in picks.into_iter().enumerate() {
                        if recent_row(ui, &file_name(&path), &note, index == 0, index == last).clicked() {
                            self.open_video(tool, PathBuf::from(path));
                        }
                    }
                }
            });
        });
    }
}
