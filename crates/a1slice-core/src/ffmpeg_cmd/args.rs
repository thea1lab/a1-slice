//! ffmpeg argument lists and the progress lines ffmpeg prints.

use crate::types::{Ms, TranscriptSegment};

use super::filters::format_srt_time;

pub fn generate_srt(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                i + 1,
                format_srt_time(seg.start_ms),
                format_srt_time(seg.end_ms),
                seg.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_ffmpeg_progress(stderr: &str) -> Option<i64> {
    let mut last = None;
    let mut rest = stderr;
    while let Some(idx) = rest.find("time=") {
        let after = &rest[idx + 5..];
        if let Some((hours, minutes, seconds, _)) = parse_hms_frac_prefix(after) {
            last = Some(hours * 3600 + minutes * 60 + seconds);
        }
        rest = &rest[idx + 5..];
    }
    last
}

pub fn parse_ffmpeg_duration(stderr: &str) -> Option<i64> {
    let mut rest = stderr;
    while let Some(idx) = rest.find("Duration:") {
        let after = &rest[idx + "Duration:".len()..];
        let trimmed = after.trim_start_matches(char::is_whitespace);
        if let Some((hours, minutes, seconds, frac)) = parse_hms_frac_prefix(trimmed) {
            let frac_value: f64 = format!("0.{frac}").parse().unwrap_or(0.0);
            let ms = (frac_value * 1000.0).round() as i64;
            return Some(hours * 3_600_000 + minutes * 60_000 + seconds * 1000 + ms);
        }
        rest = &rest[idx + "Duration:".len()..];
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoSize {
    pub width: i32,
    pub height: i32,
}

pub fn parse_ffmpeg_video_size(stderr: &str) -> Option<VideoSize> {
    let idx = stderr.find("Video:")?;
    find_wxh(&stderr[idx + "Video:".len()..])
}

fn find_wxh(text: &str) -> Option<VideoSize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut end = i + 1;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let max_take = (end - i).min(5);
            if max_take >= 2 {
                for take in (2..=max_take).rev() {
                    let x_at = i + take;
                    if bytes.get(x_at) != Some(&b'x') {
                        continue;
                    }
                    let hstart = x_at + 1;
                    let mut hend = hstart;
                    while hend < bytes.len() && bytes[hend].is_ascii_digit() && hend - hstart < 5 {
                        hend += 1;
                    }
                    if hend - hstart >= 2 {
                        let width: i32 = text[i..i + take].parse().ok()?;
                        let height: i32 = text[hstart..hend].parse().ok()?;
                        return Some(VideoSize { width, height });
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn take_digits(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    if !bytes.first().is_some_and(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    Some((&text[i..], &text[..i]))
}

fn parse_hms_frac_prefix(text: &str) -> Option<(i64, i64, i64, &str)> {
    let (text, hours) = take_digits(text)?;
    let text = text.strip_prefix(':')?;
    let (text, minutes) = take_digits(text)?;
    let text = text.strip_prefix(':')?;
    let (text, seconds) = take_digits(text)?;
    let text = text.strip_prefix('.')?;
    let (_rest, frac) = take_digits(text)?;
    Some((
        hours.parse().ok()?,
        minutes.parse().ok()?,
        seconds.parse().ok()?,
        frac,
    ))
}

fn js_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    format!("{n}")
}

pub fn extract_audio_args(video_path: &str, out_path: &str) -> Vec<String> {
    vec![
        "-i".into(),
        video_path.into(),
        "-af".into(),
        "afftdn=nr=12:nf=-20:tn=1".into(),
        "-ar".into(),
        "16000".into(),
        "-ac".into(),
        "1".into(),
        "-f".into(),
        "wav".into(),
        "-y".into(),
        out_path.into(),
    ]
}

pub fn split_wav_args(
    wav_path: &str,
    chunk_path: &str,
    offset_sec: f64,
    chunk_sec: f64,
) -> Vec<String> {
    vec![
        "-ss".into(),
        js_number(offset_sec),
        "-i".into(),
        wav_path.into(),
        "-t".into(),
        js_number(chunk_sec),
        "-ar".into(),
        "16000".into(),
        "-ac".into(),
        "1".into(),
        "-y".into(),
        chunk_path.into(),
    ]
}

pub fn build_preview_clip_args(
    video_path: &str,
    output_path: &str,
    start_sec: f64,
    duration_sec: f64,
) -> Vec<String> {
    vec![
        "-ss".into(),
        js_number(start_sec),
        "-i".into(),
        video_path.into(),
        "-t".into(),
        js_number(duration_sec),
        "-vf".into(),
        "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "18".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "192k".into(),
        "-threads".into(),
        "0".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
        "-y".into(),
        output_path.into(),
    ]
}

pub fn build_cut_clip_args(
    video_path: &str,
    output_path: &str,
    start_sec: f64,
    duration_sec: f64,
    video_filter: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "-ss".into(),
        js_number(start_sec),
        "-i".into(),
        video_path.into(),
        "-t".into(),
        js_number(duration_sec),
    ];
    if let Some(filter) = video_filter.filter(|value| !value.is_empty()) {
        args.push("-vf".into());
        args.push(filter.to_string());
    }
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "18".into(),
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
        "-y".into(),
        output_path.into(),
    ]);
    args
}

pub fn build_copy_clip_args(
    video_path: &str,
    output_path: &str,
    start_sec: f64,
    duration_sec: f64,
) -> Vec<String> {
    vec![
        "-ss".into(),
        js_number(start_sec),
        "-i".into(),
        video_path.into(),
        "-t".into(),
        js_number(duration_sec),
        "-c".into(),
        "copy".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
        "-y".into(),
        output_path.into(),
    ]
}

pub fn shift_subtitles(
    subtitles: &[TranscriptSegment],
    start_ms: Ms,
    end_ms: Ms,
) -> Vec<TranscriptSegment> {
    let duration = end_ms - start_ms;
    subtitles
        .iter()
        .filter(|segment| segment.end_ms > start_ms && segment.start_ms < end_ms)
        .map(|segment| TranscriptSegment {
            start_ms: (segment.start_ms - start_ms).max(0),
            end_ms: duration.min(segment.end_ms - start_ms),
            text: segment.text.clone(),
        })
        .collect()
}
