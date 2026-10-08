//! SRT clocks and the ffmpeg filter strings for an export.

use crate::types::{CaptionStyle, Ms, SubtitleExport};

pub fn format_srt_time(ms: Ms) -> String {
    let total_seconds = ms.div_euclid(1000);
    let hours = total_seconds.div_euclid(3600);
    let minutes = total_seconds.rem_euclid(3600) / 60;
    let seconds = total_seconds.rem_euclid(60);
    let millis = ms.rem_euclid(1000);
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

/// Escape a path for an ffmpeg filter graph argument.
pub fn escape_filter_path(file_path: &str) -> String {
    file_path
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

pub fn caption_burn_filter(
    srt_path: &str,
    fonts_dir: &str,
    font_name: &str,
    style: &CaptionStyle,
    cell_ratio: f64,
) -> String {
    let force =
        crate::captions::caption_force_style(style, font_name, cell_ratio).replace(',', "\\,");
    format!(
        "subtitles='{}':fontsdir='{}':force_style='{}'",
        escape_filter_path(srt_path),
        escape_filter_path(fonts_dir),
        force
    )
}

pub fn video_filter_for_export(
    crop_filter: Option<&str>,
    mode: SubtitleExport,
    burn_filter: Option<&str>,
) -> Option<String> {
    let burn = if mode == SubtitleExport::Burn {
        burn_filter
    } else {
        None
    };
    let crop_truthy = crop_filter.is_some_and(|value| !value.is_empty());
    let burn_truthy = burn.is_some_and(|value| !value.is_empty());
    if crop_truthy && burn_truthy {
        return Some(format!("{},{}", crop_filter.unwrap(), burn.unwrap()));
    }
    if let Some(burn) = burn {
        return Some(burn.to_string());
    }
    crop_filter.map(str::to_string)
}
