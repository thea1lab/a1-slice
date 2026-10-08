//! Clocks, caption style helpers, recent videos, and playback audio.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardState;
use eframe::egui::{self, Color32, RichText};

use super::theme::MUTED;

pub(super) fn clock(ms: i64) -> String {
    a1slice_core::preview::format_playback_clock(ms.max(0) as f64)
}

pub(super) fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

pub(super) fn settings_of(state: &WizardState) -> AppSettings {
    AppSettings {
        provider: state.provider,
        model: state.model.clone(),
        api_key: state.api_key.clone(),
        api_keys: state.api_keys.clone(),
        user_hint: state.user_hint.clone(),
        language: state.language,
        entropy_thold: state.entropy_thold,
        max_context: state.max_context,
        beam_size: state.beam_size,
        temperature_inc: state.temperature_inc,
    }
}

pub(super) fn default_style() -> CaptionStyle {
    CaptionStyle {
        color: CaptionColor::White,
        custom_color: None,
        position: CaptionPosition::Bottom,
        size: CaptionSize::Large,
        font_size: None,
        font: CaptionFont::Sans,
    }
}

pub(super) fn current_crop(state: &WizardState, ratio: CropRatio) -> ClipCrop {
    if ratio == CropRatio::Original {
        DEFAULT_CROP
    } else {
        state.framing.get(&ratio).copied().unwrap_or(ClipCrop {
            ratio,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        })
    }
}

pub(super) fn active_cues(state: &WizardState, source: CaptionSource) -> Vec<TranscriptSegment> {
    if source == CaptionSource::Manual {
        if let Some(captions) = &state.captions {
            if captions.source == CaptionSource::Manual && !captions.cues.is_empty() {
                return captions.cues.clone();
            }
        }
    }
    state.segments.clone()
}

pub(super) fn caption_project(
    source: CaptionSource,
    look: CaptionLook,
    style: CaptionStyle,
    cues: Vec<TranscriptSegment>,
    file_path: Option<String>,
) -> CaptionProject {
    CaptionProject {
        source,
        look,
        style,
        cues,
        file_path,
    }
}

fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

pub(super) fn ink_color(style: &CaptionStyle) -> Color32 {
    if let Some(hex) = style
        .custom_color
        .as_deref()
        .and_then(|h| h.strip_prefix('#'))
    {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color32::from_rgb(r, g, b);
            }
        }
    }
    caption_color(style.color)
}

fn caption_color(color: CaptionColor) -> Color32 {
    match color {
        CaptionColor::White => Color32::WHITE,
        CaptionColor::Cream => Color32::from_rgb(0xff, 0xf8, 0xe0),
        CaptionColor::Yellow => Color32::from_rgb(0xff, 0xe1, 0x4a),
        CaptionColor::Black => Color32::from_rgb(0x11, 0x11, 0x11),
    }
}

pub(super) fn style_controls(ui: &mut egui::Ui, style: &mut CaptionStyle, _look: &mut CaptionLook) {
    ui.label(RichText::new("COLOR").small().color(MUTED));
    ui.horizontal_wrapped(|ui| {
        for (color, label) in [
            (CaptionColor::White, "White"),
            (CaptionColor::Cream, "Cream"),
            (CaptionColor::Yellow, "Yellow"),
            (CaptionColor::Black, "Black"),
        ] {
            if ui
                .selectable_label(style.color == color && style.custom_color.is_none(), label)
                .clicked()
            {
                style.color = color;
                style.custom_color = None;
            }
        }
    });
    let mut custom = style.custom_color.clone().unwrap_or_default();
    if ui.text_edit_singleline(&mut custom).changed() {
        if custom.trim().is_empty() {
            style.custom_color = None;
        } else if let Some(hex) = normalize_hex(&custom) {
            style.custom_color = Some(hex);
        }
    }
    ui.label(
        RichText::new("Or type a colour, like #ffb83e.")
            .small()
            .color(MUTED),
    );
    ui.label(RichText::new("POSITION").small().color(MUTED));
    ui.horizontal(|ui| {
        for (pos, label) in [
            (CaptionPosition::Bottom, "Bottom"),
            (CaptionPosition::Middle, "Middle"),
            (CaptionPosition::Top, "Top"),
        ] {
            if ui.selectable_label(style.position == pos, label).clicked() {
                style.position = pos;
            }
        }
    });
    ui.label(RichText::new("SIZE").small().color(MUTED));
    ui.horizontal(|ui| {
        for (size, label) in [
            (CaptionSize::Small, "Small"),
            (CaptionSize::Medium, "Medium"),
            (CaptionSize::Large, "Large"),
        ] {
            if ui.selectable_label(style.size == size, label).clicked() {
                style.size = size;
                style.font_size = None;
            }
        }
    });
    ui.label(RichText::new("FONT").small().color(MUTED));
    ui.horizontal(|ui| {
        for (font, label) in [
            (CaptionFont::Sans, "Sans"),
            (CaptionFont::Serif, "Serif"),
            (CaptionFont::Mono, "Mono"),
        ] {
            if ui.selectable_label(style.font == font, label).clicked() {
                style.font = font;
            }
        }
    });
}

pub(super) fn load_recent() -> Vec<(String, String)> {
    sidecars::read_recent()
        .into_iter()
        .map(|path| {
            let info = sidecars::inspect_video(std::path::Path::new(&path));
            let mut parts = Vec::new();
            if info.has_transcript {
                parts.push("Transcript saved".to_string());
            }
            if info.has_clips {
                parts.push(if info.clip_count == 1 {
                    "1 clip".into()
                } else {
                    format!("{} clips", info.clip_count)
                });
            }
            if info.has_framing {
                parts.push("Frame saved".into());
            }
            if info.has_captions {
                parts.push("Captions saved".into());
            }
            (path, parts.join(" · "))
        })
        .collect()
}

/// End a playback process and wait until it is gone.
///
/// Dropping a `Child` leaves ffplay or mpv running, so closing the window
/// would keep the video's sound going.
pub(super) fn stop_child(mut child: std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(super) fn start_audio(path: &str, at_ms: i64, volume: f32) -> Option<std::process::Child> {
    let sec = format!("{:.3}", at_ms.max(0) as f64 / 1000.0);
    let vol = ((volume.clamp(0.0, 1.0)) * 100.0) as i32;
    if backend::which("ffplay").is_some() {
        return std::process::Command::new("ffplay")
            .args([
                "-nodisp",
                "-autoexit",
                "-ss",
                &sec,
                "-volume",
                &vol.to_string(),
                "-loglevel",
                "quiet",
                path,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    }
    if backend::which("mpv").is_some() {
        return std::process::Command::new("mpv")
            .args([
                "--no-video",
                "--really-quiet",
                &format!("--start={sec}"),
                &format!("--volume={vol}"),
                path,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{file_name, normalize_hex};

    #[test]
    fn hex_colour_gains_a_hash_and_lowercase() {
        assert_eq!(normalize_hex("  #FFb83E "), Some("#ffb83e".into()));
        assert_eq!(normalize_hex("ffb83e"), Some("#ffb83e".into()));
        assert_eq!(normalize_hex("#fff"), None);
        assert_eq!(normalize_hex("not-a-colour"), None);
    }

    #[test]
    fn file_name_keeps_the_last_path_piece() {
        assert_eq!(file_name("/tmp/interview.mp4"), "interview.mp4");
        assert_eq!(file_name("interview.mp4"), "interview.mp4");
    }

    #[test]
    fn stop_child_ends_the_process() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sleep");
        let pid = child.id();
        super::stop_child(child);
        assert!(
            std::fs::metadata(format!("/proc/{pid}")).is_err(),
            "process {pid} was still running"
        );
    }
}
