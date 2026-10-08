//! Review screen: identity, keep, and the clip list.

use a1slice_core::types::*;
use a1slice_core::wizard::WizardAction;
use eframe::egui::{self, Color32, RichText, Stroke};

use super::layout::{clip_length_words, review_frame, text_button_width};
use super::paint::paint_chevron;
use super::support::clock;
use super::theme::{CREAM, CREAM_HEAD, HAIRLINE, MUTED, ORANGE};
use super::widgets::{primary_lg, review_secondary};
use super::A1App;

impl A1App {
    pub(super) fn review(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.focus_review_clip();
        let avail_w = ui.available_width();
        let viewport_h = ui.clip_rect().height();
        let avail_h = if viewport_h.is_finite() && viewport_h > 1.0 {
            viewport_h
        } else {
            640.0
        };
        let tex = self
            .frame
            .as_ref()
            .map(|texture| texture.size_vec2())
            .unwrap_or(egui::vec2(16.0, 9.0));
        // 8px is the gap the parent layout inserts between the picture and the dock.
        let dock = if self.review_dock_px > 1.0 {
            self.review_dock_px + 8.0
        } else {
            248.0
        };
        let frame = review_frame(avail_w, avail_h, tex.x, tex.y, dock);
        let row_w = frame.button * 2.0 + frame.gap * 2.0 + frame.picture.x;
        let side = ((avail_w - row_w) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side);
            self.review_nav(ui, frame.button, frame.picture.y, -1);
            ui.add_space(frame.gap);
            ui.vertical(|ui| {
                ui.set_width(frame.picture.x);
                self.picture_limited(ui, ctx, frame.picture.x, frame.picture.y);
            });
            ui.add_space(frame.gap);
            self.review_nav(ui, frame.button, frame.picture.y, 1);
        });
        let dock_row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(side + frame.button + frame.gap);
            ui.vertical(|ui| {
                ui.set_width(frame.picture.x);
                self.review_column(ui);
            });
        });
        self.review_dock_px = dock_row.response.rect.height();
    }

    pub(super) fn review_nav(
        &mut self,
        ui: &mut egui::Ui,
        button: f32,
        video_h: f32,
        delta: isize,
    ) {
        let (column, _) = ui.allocate_exact_size(
            egui::vec2(button, video_h.max(button)),
            egui::Sense::hover(),
        );
        let len = self.state.clips.len();
        if len == 0 {
            return;
        }
        let index = self.review_index.min(len - 1);
        let enabled = if delta < 0 {
            index > 0
        } else {
            index + 1 < len
        };
        if !enabled {
            return;
        }
        let hit = egui::Rect::from_center_size(column.center(), egui::vec2(button, button));
        let response = ui.interact(
            hit,
            ui.id()
                .with(if delta < 0 { "prev-clip" } else { "next-clip" }),
            egui::Sense::CLICK,
        );
        let fill = if response.hovered() {
            Color32::from_rgb(0xcc, 0x3a, 0x05)
        } else {
            ORANGE
        };
        ui.painter().rect(
            hit,
            egui::CornerRadius::same(8),
            fill,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
        paint_chevron(ui.painter(), hit, delta < 0);
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked() {
            self.step_review(delta);
        }
        response.on_hover_text(if delta < 0 {
            "Previous clip"
        } else {
            "Next clip"
        });
    }

    pub(super) fn review_column(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.add_space(16.0);
        let (hair, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(
            hair.x_range(),
            hair.center().y,
            Stroke::new(1.0_f32, HAIRLINE),
        );
        ui.add_space(16.0);
        let kept = self.state.clips.iter().filter(|clip| clip.approved).count();
        let total = self.state.clips.len();
        if let Some(clip) = self.review_clip() {
            let index = self.review_index.min(total.saturating_sub(1));
            self.review_identity(ui, &clip, index, total, kept);
        } else {
            ui.label(
                RichText::new("No clips to review yet.")
                    .size(16.0)
                    .color(MUTED),
            );
        }
        if !self.status.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new(&self.status).color(ORANGE));
        }
        ui.add_space(20.0);
        self.review_actions(ui, kept);
        ui.add_space(16.0);
    }

    pub(super) fn review_identity(
        &self,
        ui: &mut egui::Ui,
        clip: &ClipSegmentWithStatus,
        index: usize,
        total: usize,
        kept: usize,
    ) {
        let width = ui.available_width();
        let time_size = if width < 560.0 {
            36.0
        } else if width < 900.0 {
            44.0
        } else {
            52.0
        };
        let cream_soft = Color32::from_rgb(0xd9, 0xd3, 0xc5);
        let length = clip_length_words(clip.start_ms, clip.end_ms);
        let mut length_size_px = time_size;
        let mut length_galley = ui.painter().layout_no_wrap(
            length.clone(),
            egui::FontId::new(length_size_px, egui::FontFamily::Name("Noto Serif".into())),
            CREAM_HEAD,
        );
        while length_size_px > 32.0 && length_galley.size().x > width {
            length_size_px -= 4.0;
            length_galley = ui.painter().layout_no_wrap(
                length.clone(),
                egui::FontId::new(length_size_px, egui::FontFamily::Name("Noto Serif".into())),
                CREAM_HEAD,
            );
        }
        if length_galley.size().x > width {
            length_galley = ui.painter().layout(
                length,
                egui::FontId::new(length_size_px, egui::FontFamily::Name("Noto Serif".into())),
                CREAM_HEAD,
                width,
            );
        }
        let range = format!("{} – {}", clock(clip.start_ms), clock(clip.end_ms));
        let range_galley =
            ui.painter()
                .layout_no_wrap(range, egui::FontId::proportional(15.0), cream_soft);
        let meta = format!("Clip {} of {total}  ·  {kept} kept", index + 1);
        let meta_galley =
            ui.painter()
                .layout_no_wrap(meta, egui::FontId::proportional(13.0), MUTED);
        let title_color = if clip.approved { CREAM } else { MUTED };
        let title_font = egui::FontId::proportional(16.0);
        let meta_size = meta_galley.size();
        let title_plain =
            ui.painter()
                .layout_no_wrap(clip.title.clone(), title_font.clone(), title_color);
        let meta_gap = 20.0;
        let meta_beside = title_plain.size().x + meta_gap + meta_size.x <= width;
        let title_galley = if meta_beside {
            title_plain
        } else {
            ui.painter()
                .layout(clip.title.clone(), title_font, title_color, width)
        };
        let now_size = length_galley.size();
        let range_size = range_galley.size();
        let title_size = title_galley.size();
        let beside_gap = 28.0;
        let range_beside = now_size.x + beside_gap + range_size.x <= width;
        let time_h = if range_beside {
            now_size.y
        } else {
            now_size.y + 4.0 + range_size.y
        };
        let gap_title = 12.0;
        let name_h = if meta_beside {
            title_size.y.max(meta_size.y)
        } else {
            title_size.y + 4.0 + meta_size.y
        };
        let height = time_h + gap_title + name_h;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter();
        painter.galley(rect.left_top(), length_galley, CREAM_HEAD);
        let range_pos = if range_beside {
            egui::pos2(
                rect.left() + now_size.x + beside_gap,
                rect.top() + (now_size.y - range_size.y) * 0.5,
            )
        } else {
            egui::pos2(rect.left(), rect.top() + now_size.y + 4.0)
        };
        painter.galley(range_pos, range_galley, cream_soft);
        let title_y = rect.top() + time_h + gap_title;
        painter.galley(egui::pos2(rect.left(), title_y), title_galley, title_color);
        let meta_pos = if meta_beside {
            egui::pos2(
                rect.left() + title_size.x + meta_gap,
                title_y + (title_size.y - meta_size.y) * 0.5,
            )
        } else {
            egui::pos2(rect.left(), title_y + title_size.y + 4.0)
        };
        painter.galley(meta_pos, meta_galley, MUTED);
    }

    pub(super) fn review_actions(&mut self, ui: &mut egui::Ui, kept: usize) {
        let width = ui.available_width();
        let export_label = format!("Export {kept} {}", if kept == 1 { "clip" } else { "clips" });
        let export_w = text_button_width(ui, &export_label, 16.0, 22.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.set_min_width(width);
            if review_secondary(ui, "Find again").clicked() {
                self.dispatch(WizardAction::ShowScreen(Screen::Find));
            }
            if review_secondary(ui, "Add a part").clicked() {
                self.dispatch(WizardAction::AddClip);
                self.review_index = self.state.clips.len().saturating_sub(1);
                self.review_for = None;
                self.persist_clips();
                self.focus_review_clip();
            }
            if let Some(clip) = self.review_clip() {
                let keep = if clip.approved {
                    "Drop this one"
                } else {
                    "Keep this one"
                };
                if review_secondary(ui, keep).clicked() {
                    self.toggle_review_clip();
                }
            }
            let spare = ui.available_width() - export_w - 8.0;
            if spare > 8.0 {
                ui.add_space(spare);
            }
            if primary_lg(ui, &export_label).clicked() {
                self.start_export_clips();
            }
        });
    }
}
