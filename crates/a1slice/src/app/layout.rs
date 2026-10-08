//! Player and review-bar geometry. Pure enough to unit test.

use a1slice_core::clips::ClipViewWindow;
use eframe::egui::{self, Color32};

use super::support::clock;
use super::theme::CREAM_HEAD;

pub(super) struct TransportSlots {
    pub(super) play: egui::Rect,
    pub(super) seek: egui::Rect,
    pub(super) time: egui::Rect,
    pub(super) speaker: egui::Rect,
    pub(super) slider: egui::Rect,
}

pub(super) struct SeekHit {
    pub(super) ms: i64,
    pub(super) dragging: bool,
}

pub(super) fn transport_slots(bar: egui::Rect, time_width: f32) -> TransportSlots {
    let pad = 10.0;
    let gap = 8.0;
    let play_s = 40.0_f32.min((bar.height() - 8.0).max(24.0));
    let speaker_s = 36.0_f32.min(play_s);
    let full_slider = 68.0;
    let min_seek = 48.0;
    let with_slider = pad * 2.0
        + play_s
        + gap
        + min_seek
        + gap
        + time_width
        + gap
        + speaker_s
        + gap
        + full_slider;
    let slider_w = if bar.width() >= with_slider {
        full_slider
    } else {
        0.0
    };
    let mid = bar.center().y;
    let mut right = bar.right() - pad;
    let slider = if slider_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - slider_w, mid - 14.0),
            egui::pos2(right, mid + 14.0),
        );
        right = rect.left() - 4.0;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let speaker = egui::Rect::from_center_size(
        egui::pos2(right - speaker_s * 0.5, mid),
        egui::vec2(speaker_s, speaker_s),
    );
    right = speaker.left() - gap;
    let time = egui::Rect::from_min_max(
        egui::pos2(right - time_width, mid - 12.0),
        egui::pos2(right, mid + 12.0),
    );
    let play = egui::Rect::from_center_size(
        egui::pos2(bar.left() + pad + play_s * 0.5, mid),
        egui::vec2(play_s, play_s),
    );
    let seek_left = play.right() + gap;
    let seek = egui::Rect::from_min_max(
        egui::pos2(seek_left, bar.top()),
        egui::pos2((time.left() - gap).max(seek_left), bar.bottom()),
    );
    TransportSlots {
        play,
        seek,
        time,
        speaker,
        slider,
    }
}

pub(super) struct ReviewSlots {
    pub(super) play: egui::Rect,
    pub(super) start_label: egui::Rect,
    pub(super) seek: egui::Rect,
    pub(super) end_label: egui::Rect,
    pub(super) time: egui::Rect,
    pub(super) speaker: egui::Rect,
    pub(super) slider: egui::Rect,
}

pub(super) fn review_slots(
    bar: egui::Rect,
    mut start_w: f32,
    mut end_w: f32,
    time_w: f32,
) -> ReviewSlots {
    let pad = 10.0;
    let gap = 8.0;
    let play_s = 40.0_f32.min((bar.height() - 8.0).max(24.0));
    let speaker_s = 36.0_f32.min(play_s);
    let full_slider = 68.0;
    let min_seek = 48.0;
    let label_extra = |start: f32, end: f32| {
        let mut extra = 0.0;
        if start > 0.0 {
            extra += gap + start;
        }
        if end > 0.0 {
            extra += gap + end;
        }
        extra
    };
    let fixed = pad * 2.0 + play_s + gap + min_seek + gap + time_w + gap + speaker_s;
    let fits = |start: f32, end: f32, slider: f32| {
        fixed + label_extra(start, end) + if slider > 0.0 { gap + slider } else { 0.0 }
    };
    let mut slider_w = full_slider;
    if bar.width() < fits(start_w, end_w, slider_w) {
        slider_w = 0.0;
    }
    if bar.width() < fits(start_w, end_w, slider_w) {
        start_w = 0.0;
        end_w = 0.0;
    }
    if slider_w == 0.0 && bar.width() >= fits(start_w, end_w, full_slider) {
        slider_w = full_slider;
    }

    let mid = bar.center().y;
    let mut right = bar.right() - pad;
    let slider = if slider_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - slider_w, mid - 14.0),
            egui::pos2(right, mid + 14.0),
        );
        right = rect.left() - 4.0;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let speaker = egui::Rect::from_center_size(
        egui::pos2(right - speaker_s * 0.5, mid),
        egui::vec2(speaker_s, speaker_s),
    );
    right = speaker.left() - gap;
    let end_label = if end_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(right - end_w, mid - 12.0),
            egui::pos2(right, mid + 12.0),
        );
        right = rect.left() - gap;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(right, mid), egui::pos2(right, mid))
    };
    let play = egui::Rect::from_center_size(
        egui::pos2(bar.left() + pad + play_s * 0.5, mid),
        egui::vec2(play_s, play_s),
    );
    let mut left = play.right() + gap;
    let time = egui::Rect::from_min_max(
        egui::pos2(left, mid - 12.0),
        egui::pos2(left + time_w, mid + 12.0),
    );
    left = time.right() + gap;
    let start_label = if start_w > 0.0 {
        let rect = egui::Rect::from_min_max(
            egui::pos2(left, mid - 12.0),
            egui::pos2(left + start_w, mid + 12.0),
        );
        left = rect.right() + gap;
        rect
    } else {
        egui::Rect::from_min_max(egui::pos2(left, mid), egui::pos2(left, mid))
    };
    let seek = egui::Rect::from_min_max(
        egui::pos2(left, bar.top()),
        egui::pos2(right.max(left), bar.bottom()),
    );
    ReviewSlots {
        play,
        start_label,
        seek,
        end_label,
        time,
        speaker,
        slider,
    }
}

pub(super) fn mono_clock_width(ui: &egui::Ui, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let digit = (0..10)
        .map(|n| {
            ui.painter()
                .layout_no_wrap(n.to_string(), font.clone(), CREAM_HEAD)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let colon = ui
        .painter()
        .layout_no_wrap(":".to_string(), font, CREAM_HEAD)
        .size()
        .x;
    clock(duration_ms).chars().fold(0.0, |width, ch| {
        width + if ch.is_ascii_digit() { digit } else { colon }
    })
}

pub(super) fn player_time_width(ui: &egui::Ui, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let slash = ui
        .painter()
        .layout_no_wrap(" / ".to_string(), font, CREAM_HEAD)
        .size()
        .x;
    (mono_clock_width(ui, duration_ms) * 2.0 + slash).ceil()
}

pub(super) fn edge_label_width(ui: &egui::Ui, prefix: &str, duration_ms: i64) -> f32 {
    let font = egui::FontId::proportional(13.0);
    let prefix_w = ui
        .painter()
        .layout_no_wrap(prefix.to_string(), font, CREAM_HEAD)
        .size()
        .x;
    (prefix_w + mono_clock_width(ui, duration_ms)).ceil()
}

pub(super) fn ms_to_x(ms: i64, view: ClipViewWindow, left: f32, right: f32) -> f32 {
    let span = (view.view_end - view.view_start).max(1) as f32;
    let t = (ms - view.view_start) as f32 / span;
    egui::lerp(left..=right, t.clamp(0.0, 1.0))
}

pub(super) struct ReviewFrame {
    pub(super) picture: egui::Vec2,
    pub(super) button: f32,
    pub(super) gap: f32,
}

pub(super) fn review_frame(
    avail_w: f32,
    avail_h: f32,
    tex_w: f32,
    tex_h: f32,
    dock: f32,
) -> ReviewFrame {
    let button = if avail_w < 720.0 { 56.0 } else { 76.0 };
    let gap = 24.0;
    let margin = 24.0;
    // Leave the measured dock on screen. The slack stops a one-pixel overrun from opening a scrollbar.
    let max_w = (avail_w - 2.0 * (button + gap + margin)).max(160.0);
    let max_h = (avail_h - dock - VIEW_SLACK).max(180.0);
    ReviewFrame {
        picture: fit_picture(tex_w, tex_h, max_w, max_h),
        button,
        gap,
    }
}

/// Pixels kept free under the picture so the content stays shorter than the viewport.
///
/// A scrollbar that appears for a one-pixel overrun covers the right edge, and it
/// stays there when the window grows because the picture grows by the same amount.
pub(super) const VIEW_SLACK: f32 = 16.0;

/// Visible height inside the scroll area.
///
/// `clip_rect` is taller than the viewport by the clip margin, so sizing from it
/// always overflows. `available_height` is the scroll viewport at the start of the page.
pub(super) fn viewport_height(ui: &egui::Ui) -> f32 {
    let height = ui.available_height();
    if height.is_finite() && height > 1.0 {
        height
    } else {
        640.0
    }
}

/// Picture height that leaves `dock` pixels for the controls under it.
pub(super) fn picture_room(viewport: f32, dock: f32, min_picture: f32) -> f32 {
    (viewport - dock - VIEW_SLACK).max(min_picture)
}

pub(super) fn fit_picture(tex_w: f32, tex_h: f32, max_w: f32, max_h: f32) -> egui::Vec2 {
    let tex_w = tex_w.max(1.0);
    let tex_h = tex_h.max(1.0);
    let max_w = max_w.max(1.0);
    let max_h = max_h.max(1.0);
    let mut width = max_w;
    let mut height = width * tex_h / tex_w;
    if height > max_h {
        height = max_h;
        width = height * tex_w / tex_h;
    }
    egui::vec2(width, height)
}

/// After a start or end drag, playback stays inside the clip and the loop stays on.
pub(super) fn playhead_after_trim(
    resume_ms: i64,
    preview_ms: i64,
    start_ms: i64,
    end_ms: i64,
    playing: bool,
) -> i64 {
    let end_ms = end_ms.max(start_ms);
    if playing {
        if resume_ms >= start_ms && resume_ms < end_ms {
            resume_ms
        } else {
            start_ms
        }
    } else if preview_ms >= end_ms {
        end_ms.saturating_sub(1).max(start_ms)
    } else if preview_ms < start_ms {
        start_ms
    } else {
        preview_ms
    }
}

pub(super) fn clip_length_words(start_ms: i64, end_ms: i64) -> String {
    let total = (end_ms - start_ms).max(0) / 1000;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(count_words(hours, "hour", "hours"));
    }
    if minutes > 0 {
        parts.push(count_words(minutes, "minute", "minutes"));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(count_words(seconds, "second", "seconds"));
    }
    match parts.len() {
        1 => parts.remove(0),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().expect("at least one part");
            format!("{} and {last}", parts.join(", "))
        }
    }
}

fn count_words(count: i64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

pub(super) fn text_button_width(ui: &egui::Ui, label: &str, text_size: f32, pad_x: f32) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(text_size),
        Color32::WHITE,
    );
    galley.size().x + pad_x * 2.0
}

#[cfg(test)]
mod tests {
    use super::{
        clip_length_words, fit_picture, ms_to_x, picture_room, playhead_after_trim, review_frame,
        review_slots, transport_slots, ClipViewWindow,
    };

    #[test]
    fn transport_row_is_play_seek_time_then_volume() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 56.0));
        let slots = transport_slots(bar, 72.0);
        assert!(slots.play.right() < slots.seek.left());
        assert!(slots.seek.right() <= slots.time.left());
        assert!(slots.time.right() < slots.speaker.left());
        assert!(slots.speaker.right() < slots.slider.left());
        assert!(slots.slider.width() > 0.0);
        assert!(slots.seek.width() > 200.0);
        assert!(slots.play.left() >= bar.left());
        assert!(slots.slider.right() <= bar.right());
        let mid = bar.center().y;
        for center in [
            slots.play.center().y,
            slots.seek.center().y,
            slots.time.center().y,
            slots.speaker.center().y,
            slots.slider.center().y,
        ] {
            assert!((center - mid).abs() < 0.5);
        }
    }

    #[test]
    fn review_row_places_handles_between_the_times() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 56.0));
        let slots = review_slots(bar, 80.0, 70.0, 48.0);
        assert!(slots.play.right() < slots.time.left());
        assert!(slots.time.right() <= slots.start_label.left());
        assert!(slots.start_label.right() <= slots.seek.left());
        assert!(slots.seek.right() <= slots.end_label.left());
        assert!(slots.end_label.right() < slots.speaker.left());
        assert!(slots.speaker.right() < slots.slider.left());
        assert!(slots.seek.width() > 48.0);
        assert!(slots.slider.width() > 0.0);
    }

    #[test]
    fn narrow_review_row_drops_the_time_labels() {
        let bar = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(280.0, 56.0));
        let slots = review_slots(bar, 80.0, 70.0, 48.0);
        assert_eq!(slots.start_label.width(), 0.0);
        assert_eq!(slots.end_label.width(), 0.0);
        assert_eq!(slots.slider.width(), 0.0);
        assert!(slots.seek.width() > 8.0);
        assert!(slots.play.right() < slots.time.left());
        assert!(slots.time.right() <= slots.seek.left());
        assert!(slots.seek.right() <= slots.speaker.left());
    }

    #[test]
    fn picture_room_keeps_the_controls_inside_the_window() {
        assert_eq!(picture_room(900.0, 300.0, 180.0), 584.0);
        assert_eq!(picture_room(400.0, 300.0, 180.0), 180.0);
        let window = 960.0;
        let header = 48.0;
        let dock = 320.0;
        let picture = picture_room(window - header, dock, 200.0);
        assert!(picture + dock + header <= window);
        let taller = picture_room(window - header + 200.0, dock, 200.0);
        assert!(taller > picture);
        assert!(taller + dock + header <= window + 200.0);
    }

    #[test]
    fn review_picture_grows_with_the_window() {
        let wide = review_frame(1600.0, 900.0, 1920.0, 1080.0, 248.0);
        assert!(wide.picture.y > 460.0);
        assert!(wide.picture.x > 960.0);
        assert!(wide.button >= 72.0);
        let row = wide.button * 2.0 + wide.gap * 2.0 + wide.picture.x;
        assert!(row < 1600.0);
        let narrow = review_frame(1100.0, 700.0, 1920.0, 1080.0, 248.0);
        assert!(wide.picture.x > narrow.picture.x);
        assert!(wide.picture.y > narrow.picture.y);
        let fitted = fit_picture(1920.0, 1080.0, wide.picture.x, wide.picture.y);
        assert!((fitted.x - wide.picture.x).abs() < 0.5);
        assert!((fitted.y - wide.picture.y).abs() < 0.5);
    }

    #[test]
    fn dragging_the_end_keeps_playback_inside_the_clip() {
        let start = 3_460;
        let end = 25_710;
        let resume = 12_000;
        let preview = end;
        assert_eq!(
            playhead_after_trim(resume, preview, start, end, true),
            resume
        );
        let pulled_in = 8_000;
        assert_eq!(
            playhead_after_trim(resume, pulled_in, start, pulled_in, true),
            start
        );
        assert_eq!(
            playhead_after_trim(resume, preview, start, end, false),
            end - 1
        );
        assert!(playhead_after_trim(resume, preview, start, end, false) < end);
    }

    #[test]
    fn clip_handle_sits_on_its_time() {
        let view = ClipViewWindow {
            view_start: 0,
            view_end: 10_000,
        };
        let start = ms_to_x(2_000, view, 0.0, 100.0);
        let end = ms_to_x(8_000, view, 0.0, 100.0);
        assert!((start - 20.0).abs() < 0.1);
        assert!((end - 80.0).abs() < 0.1);
        assert!(start < end);
    }

    #[test]
    fn narrow_transport_keeps_mute_and_drops_the_slider() {
        let bar = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(280.0, 56.0));
        let slots = transport_slots(bar, 90.0);
        assert_eq!(slots.slider.width(), 0.0);
        assert!(slots.play.right() < slots.seek.left());
        assert!(slots.seek.right() <= slots.time.left());
        assert!(slots.time.right() < slots.speaker.left());
        assert!(slots.speaker.right() <= bar.right());
        assert!(slots.seek.width() > 8.0);
    }

    #[test]
    fn clip_length_is_spoken_in_words() {
        assert_eq!(clip_length_words(0, 0), "0 seconds");
        assert_eq!(clip_length_words(0, 1_000), "1 second");
        assert_eq!(clip_length_words(0, 59_000), "59 seconds");
        assert_eq!(clip_length_words(0, 60_000), "1 minute");
        assert_eq!(clip_length_words(0, 75_000), "1 minute and 15 seconds");
        assert_eq!(clip_length_words(0, 121_000), "2 minutes and 1 second");
        assert_eq!(clip_length_words(0, 3_600_000), "1 hour");
        assert_eq!(
            clip_length_words(0, 3_600_000 + 15_000),
            "1 hour and 15 seconds"
        );
        assert_eq!(
            clip_length_words(0, 7_200_000 + 120_000 + 5_000),
            "2 hours, 2 minutes and 5 seconds"
        );
    }
}
