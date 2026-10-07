//! FFmpeg argument builders and stderr parsers. Does not spawn ffmpeg.

use std::path::{Path, PathBuf};

use crate::types::{CaptionFont, CaptionStyle, Ms, SubtitleExport, TranscriptSegment};

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

struct FontFile {
    file: &'static str,
    name: &'static str,
}

const SANS_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSans-Medium.ttf",
        name: "Noto Sans Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        name: "Noto Sans",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        name: "Liberation Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        name: "Liberation Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        name: "DejaVu Sans",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        name: "FreeSans",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Arial.ttf",
        name: "Arial",
    },
    FontFile {
        file: "/Library/Fonts/Arial.ttf",
        name: "Arial",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\arial.ttf",
        name: "Arial",
    },
];

const SERIF_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSerif-Medium.ttf",
        name: "Noto Serif Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
        name: "Noto Serif",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationSerif-Regular.ttf",
        name: "Liberation Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
        name: "Liberation Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
        name: "DejaVu Serif",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeSerif.ttf",
        name: "FreeSerif",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        name: "Times New Roman",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\times.ttf",
        name: "Times New Roman",
    },
];

const MONO_FILES: &[FontFile] = &[
    FontFile {
        file: "/usr/share/fonts/noto/NotoSansMono-Medium.ttf",
        name: "Noto Sans Mono Medium",
    },
    FontFile {
        file: "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        name: "Noto Sans Mono",
    },
    FontFile {
        file: "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        name: "Liberation Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        name: "Liberation Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        name: "DejaVu Sans Mono",
    },
    FontFile {
        file: "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
        name: "FreeMono",
    },
    FontFile {
        file: "/System/Library/Fonts/Supplemental/Courier New.ttf",
        name: "Courier New",
    },
    FontFile {
        file: "C:\\Windows\\Fonts\\consola.ttf",
        name: "Consolas",
    },
];

fn system_fonts(face: CaptionFont) -> &'static [FontFile] {
    match face {
        CaptionFont::Sans => SANS_FILES,
        CaptionFont::Serif => SERIF_FILES,
        CaptionFont::Mono => MONO_FILES,
    }
}

fn bundled_faces(face: CaptionFont) -> &'static [FontFile] {
    match face {
        CaptionFont::Sans => &[
            FontFile {
                file: "NotoSans-Medium.ttf",
                name: "Noto Sans Medium",
            },
            FontFile {
                file: "NotoSans-Regular.ttf",
                name: "Noto Sans",
            },
        ],
        CaptionFont::Serif => &[
            FontFile {
                file: "NotoSerif-Medium.ttf",
                name: "Noto Serif Medium",
            },
            FontFile {
                file: "NotoSerif-Regular.ttf",
                name: "Noto Serif",
            },
        ],
        CaptionFont::Mono => &[
            FontFile {
                file: "NotoSansMono-Medium.ttf",
                name: "Noto Sans Mono Medium",
            },
            FontFile {
                file: "NotoSansMono-Regular.ttf",
                name: "Noto Sans Mono",
            },
        ],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionFontMatch {
    pub dir: String,
    pub name: String,
    pub file: String,
    /// (usWinAscent + usWinDescent) / unitsPerEm. 1 when the face could not be read.
    pub cell_ratio: f64,
}

pub fn find_caption_font(face: CaptionFont, extra_dirs: &[&str]) -> Option<CaptionFontMatch> {
    let mut order = Vec::new();
    push_bundled(&mut order, face, extra_dirs);
    push_system(&mut order, face);
    push_bundled(&mut order, CaptionFont::Sans, extra_dirs);
    push_system(&mut order, CaptionFont::Sans);
    for (file, name) in order {
        if file.as_os_str().is_empty() || !file.exists() {
            continue;
        }
        let file_str = path_to_string(&file);
        return Some(CaptionFontMatch {
            dir: dir_name(&file),
            name: name.to_string(),
            file: file_str.clone(),
            cell_ratio: font_cell_ratio(&file_str),
        });
    }
    None
}

fn push_bundled(out: &mut Vec<(PathBuf, &'static str)>, face: CaptionFont, extra_dirs: &[&str]) {
    for dir in extra_dirs {
        if dir.is_empty() {
            continue;
        }
        for face_file in bundled_faces(face) {
            out.push((Path::new(dir).join(face_file.file), face_file.name));
        }
    }
}

fn push_system(out: &mut Vec<(PathBuf, &'static str)>, face: CaptionFont) {
    for face_file in system_fonts(face) {
        out.push((PathBuf::from(face_file.file), face_file.name));
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn dir_name(path: &Path) -> String {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => path_to_string(parent),
        _ => ".".to_string(),
    }
}

/// libass sizes by the Windows cell. CSS sizes by the em. Returns 1 if the face cannot be read.
pub fn font_cell_ratio(file_path: &str) -> f64 {
    match std::fs::read(file_path) {
        Ok(data) => cell_ratio_from_sfnt(&data),
        Err(_) => 1.0,
    }
}

fn cell_ratio_from_sfnt(data: &[u8]) -> f64 {
    if data.len() < 12 {
        return 1.0;
    }
    let scaler = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    const TRUETYPE: u32 = 0x0001_0000;
    const TRUE_TAG: u32 = 0x7472_7565;
    const OTTO: u32 = 0x4F54_544F;
    if scaler != TRUETYPE && scaler != TRUE_TAG && scaler != OTTO {
        return 1.0;
    }
    let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
    let mut head: Option<(usize, usize)> = None;
    let mut os2: Option<(usize, usize)> = None;
    for i in 0..num_tables {
        let Some(rec) = 12usize.checked_add(i.saturating_mul(16)) else {
            return 1.0;
        };
        if rec
            .checked_add(16)
            .map(|end| end > data.len())
            .unwrap_or(true)
        {
            return 1.0;
        }
        let tag = &data[rec..rec + 4];
        let offset = u32::from_be_bytes(data[rec + 8..rec + 12].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(data[rec + 12..rec + 16].try_into().unwrap()) as usize;
        if tag == b"head" {
            head = Some((offset, length));
        }
        if tag == b"OS/2" {
            os2 = Some((offset, length));
        }
    }
    let Some((head_off, head_len)) = head else {
        return 1.0;
    };
    let Some((os2_off, os2_len)) = os2 else {
        return 1.0;
    };
    if head_len < 20 || os2_len < 78 {
        return 1.0;
    }
    if head_off
        .checked_add(20)
        .map(|end| end > data.len())
        .unwrap_or(true)
        || os2_off
            .checked_add(78)
            .map(|end| end > data.len())
            .unwrap_or(true)
    {
        return 1.0;
    }
    let units_per_em = u16::from_be_bytes(data[head_off + 18..head_off + 20].try_into().unwrap());
    let win_ascent = u16::from_be_bytes(data[os2_off + 74..os2_off + 76].try_into().unwrap());
    let win_descent = u16::from_be_bytes(data[os2_off + 76..os2_off + 78].try_into().unwrap());
    if units_per_em == 0 {
        return 1.0;
    }
    let ratio = (win_ascent as f64 + win_descent as f64) / units_per_em as f64;
    if !ratio.is_finite() || ratio < 0.5 || ratio > 2.5 {
        return 1.0;
    }
    ratio
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::captions::DEFAULT_CAPTION_STYLE;
    use crate::types::{CaptionColor, CaptionPosition, CaptionSize};

    fn cue(start_ms: Ms, end_ms: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    fn arg_after<'a>(args: &'a [String], flag: &str) -> &'a str {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        &args[index + 1]
    }

    #[test]
    fn format_srt_time_zero() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
    }

    #[test]
    fn format_srt_time_milliseconds_only() {
        assert_eq!(format_srt_time(500), "00:00:00,500");
    }

    #[test]
    fn format_srt_time_seconds_and_millis() {
        assert_eq!(format_srt_time(5123), "00:00:05,123");
    }

    #[test]
    fn format_srt_time_minutes() {
        assert_eq!(format_srt_time(90000), "00:01:30,000");
    }

    #[test]
    fn format_srt_time_hours() {
        assert_eq!(format_srt_time(3661001), "01:01:01,001");
    }

    #[test]
    fn escape_filter_path_rewrites_slashes_colons_and_quotes() {
        assert_eq!(
            escape_filter_path("C:\\Windows\\Fonts\\arial.ttf"),
            "C\\:/Windows/Fonts/arial.ttf"
        );
        assert_eq!(escape_filter_path("/tmp/it's.srt"), "/tmp/it\\'s.srt");
    }

    #[test]
    fn generate_srt_empty() {
        assert_eq!(generate_srt(&[]), "");
    }

    #[test]
    fn generate_srt_one_segment() {
        let result = generate_srt(&[cue(0, 2000, "Hello world")]);
        assert_eq!(result, "1\n00:00:00,000 --> 00:00:02,000\nHello world\n");
    }

    #[test]
    fn generate_srt_multiple_segments() {
        let result = generate_srt(&[cue(0, 2000, "First"), cue(2000, 5000, "Second")]);
        assert_eq!(
            result,
            "1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n"
        );
    }

    #[test]
    fn generate_srt_trims_whitespace() {
        let result = generate_srt(&[cue(0, 1000, "  spaced  ")]);
        assert!(result.contains("spaced"));
        assert!(!result.contains("  spaced  "));
    }

    #[test]
    fn parse_ffmpeg_progress_returns_the_last_time() {
        let stderr =
            "frame=1 time=00:00:00.04 bitrate=N/A\nframe=40 time=00:00:03.20 bitrate=N/A\n";
        assert_eq!(parse_ffmpeg_progress(stderr), Some(3));
    }

    #[test]
    fn parse_ffmpeg_progress_missing() {
        assert_eq!(parse_ffmpeg_progress("frame=1 fps=30"), None);
    }

    #[test]
    fn parse_ffmpeg_duration_centiseconds() {
        assert_eq!(
            parse_ffmpeg_duration("Duration: 00:01:23.45, start: 0.000000"),
            Some(83450)
        );
    }

    #[test]
    fn parse_ffmpeg_duration_milliseconds() {
        assert_eq!(
            parse_ffmpeg_duration("  Duration: 01:00:00.500\n"),
            Some(3600500)
        );
    }

    #[test]
    fn parse_ffmpeg_duration_missing() {
        assert_eq!(parse_ffmpeg_duration("no duration here"), None);
    }

    #[test]
    fn parse_ffmpeg_video_size_from_stream_line() {
        let stderr = "Stream #0:0(und): Video: h264 (High) (avc1 / 0x31637661), yuv420p, 1920x1080 [SAR 1:1 DAR 16:9]";
        assert_eq!(
            parse_ffmpeg_video_size(stderr),
            Some(VideoSize {
                width: 1920,
                height: 1080
            })
        );
    }

    #[test]
    fn parse_ffmpeg_video_size_missing() {
        assert_eq!(parse_ffmpeg_video_size("Duration: 00:01:00.00"), None);
    }

    #[test]
    fn build_cut_clip_args_reencodes() {
        let args = build_cut_clip_args("/in.mp4", "/out.mp4", 12.5, 30.0, None);
        assert!(!args.iter().any(|arg| arg == "copy"));
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "aac"));
        assert_eq!(arg_after(&args, "-ss"), "12.5");
        assert_eq!(arg_after(&args, "-t"), "30");
        assert_eq!(
            args,
            vec![
                "-ss",
                "12.5",
                "-i",
                "/in.mp4",
                "-t",
                "30",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "18",
                "-c:a",
                "aac",
                "-movflags",
                "+faststart",
                "-avoid_negative_ts",
                "make_zero",
                "-y",
                "/out.mp4",
            ]
        );
    }

    #[test]
    fn build_cut_clip_args_adds_filter() {
        let filter = "crop=608:1080:656:0,scale=1080:1920";
        let args = build_cut_clip_args("/in.mp4", "/out.mp4", 0.0, 5.0, Some(filter));
        assert_eq!(arg_after(&args, "-vf"), filter);
    }

    #[test]
    fn caption_burn_filter_for_large_white_bottom() {
        let mut style = DEFAULT_CAPTION_STYLE;
        style.size = CaptionSize::Large;
        let burn =
            caption_burn_filter("/tmp/a.srt", "/usr/share/fonts", "DejaVu Sans", &style, 1.0);
        assert!(burn.contains("subtitles="));
        assert!(burn.contains("FontSize=28"));
        assert!(burn.contains("Outline=0.55"));
        assert!(burn.contains("PrimaryColour=&H00FFFFFF"));
        assert!(burn.contains("Alignment=2"));
        assert!(burn.contains("MarginV=90"));
        let scaled = caption_burn_filter(
            "/tmp/a.srt",
            "/usr/share/fonts",
            "DejaVu Sans",
            &style,
            1.618,
        );
        assert!(scaled.contains("FontSize=45"));
        assert!(scaled.contains("MarginV=90"));
        assert_eq!(
            video_filter_for_export(Some("crop=1:1:0:0"), SubtitleExport::Burn, Some(&burn))
                .as_deref(),
            Some(format!("crop=1:1:0:0,{burn}")).as_deref()
        );
        assert_eq!(
            video_filter_for_export(Some("crop=1:1:0:0"), SubtitleExport::Srt, Some(&burn))
                .as_deref(),
            Some("crop=1:1:0:0")
        );
        assert_eq!(
            video_filter_for_export(None, SubtitleExport::Off, Some(&burn)),
            None
        );
    }

    #[test]
    fn caption_burn_filter_writes_colour_place_and_size() {
        let style = CaptionStyle {
            color: CaptionColor::Yellow,
            custom_color: None,
            position: CaptionPosition::Top,
            size: CaptionSize::Small,
            font_size: None,
            font: CaptionFont::Serif,
        };
        let burn = caption_burn_filter("/tmp/a.srt", "/fonts", "Noto Serif", &style, 1.0);
        assert!(burn.contains("FontName=Noto Serif"));
        assert!(burn.contains("FontSize=18"));
        assert!(burn.contains("PrimaryColour=&H004AE1FF"));
        assert!(burn.contains("Alignment=8"));
        assert!(burn.contains("MarginV=36"));
    }

    #[test]
    fn find_caption_font_prefers_bundled_face() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("NotoSerif-Regular.ttf");
        std::fs::write(&file, b"").unwrap();
        let got = find_caption_font(CaptionFont::Serif, &[dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(got.dir, dir.path().to_str().unwrap());
        assert_eq!(got.name, "Noto Serif");
        assert_eq!(got.file, file.to_str().unwrap());
        assert_eq!(got.cell_ratio, 1.0);
    }

    #[test]
    fn find_caption_font_prefers_medium_face() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("NotoSansMono-Regular.ttf"), b"").unwrap();
        let medium = dir.path().join("NotoSansMono-Medium.ttf");
        std::fs::write(&medium, b"").unwrap();
        let got = find_caption_font(CaptionFont::Mono, &[dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(got.name, "Noto Sans Mono Medium");
        assert_eq!(got.file, medium.to_str().unwrap());
    }

    #[test]
    fn font_cell_ratio_of_noto_when_installed() {
        let mono = "/usr/share/fonts/noto/NotoSansMono-Medium.ttf";
        let sans = "/usr/share/fonts/noto/NotoSans-Medium.ttf";
        if Path::new(mono).exists() {
            assert!((font_cell_ratio(mono) - 1.618).abs() < 5e-4);
        }
        if Path::new(sans).exists() {
            assert!((font_cell_ratio(sans) - 1.519).abs() < 5e-4);
        }
    }

    #[test]
    fn font_cell_ratio_reads_dejavu_when_installed() {
        let path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
        if Path::new(path).exists() {
            assert!((font_cell_ratio(path) - 1.1640625).abs() < 1e-9);
        }
    }

    #[test]
    fn font_cell_ratio_missing_file_is_one() {
        assert_eq!(font_cell_ratio("/no/such/font.ttf"), 1.0);
    }

    #[test]
    fn build_preview_clip_args_reencodes_yuv420p() {
        let args = build_preview_clip_args("/in.mp4", "/preview.mp4", 5.0, 8.0);
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "veryfast"));
        assert!(args.iter().any(|arg| arg == "yuv420p"));
        assert_eq!(
            arg_after(&args, "-vf"),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p"
        );
        assert!(args.iter().any(|arg| arg.contains("scale=")));
        assert!(!args.iter().any(|arg| arg == "copy"));
        assert_eq!(arg_after(&args, "-ss"), "5");
        assert_eq!(arg_after(&args, "-t"), "8");
    }

    #[test]
    fn build_copy_clip_args_stream_copies() {
        assert_eq!(
            build_copy_clip_args("/in.mp4", "/out.mp4", 1.25, 4.0),
            vec![
                "-ss",
                "1.25",
                "-i",
                "/in.mp4",
                "-t",
                "4",
                "-c",
                "copy",
                "-avoid_negative_ts",
                "make_zero",
                "-y",
                "/out.mp4",
            ]
        );
    }

    #[test]
    fn extract_audio_and_split_wav_args() {
        assert_eq!(
            extract_audio_args("/in.mp4", "/tmp/out.wav"),
            vec![
                "-i",
                "/in.mp4",
                "-af",
                "afftdn=nr=12:nf=-20:tn=1",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-f",
                "wav",
                "-y",
                "/tmp/out.wav",
            ]
        );
        assert_eq!(
            split_wav_args("/in.wav", "/in-chunk0.wav", 0.0, 300.0),
            vec![
                "-ss",
                "0",
                "-i",
                "/in.wav",
                "-t",
                "300",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-y",
                "/in-chunk0.wav",
            ]
        );
    }

    #[test]
    fn shift_subtitles_clips_to_the_window() {
        let cues = vec![
            cue(0, 500, "before"),
            cue(500, 1500, "overlap"),
            cue(2000, 2500, "inside"),
            cue(3000, 4000, "after"),
        ];
        let shifted = shift_subtitles(&cues, 1000, 3000);
        assert_eq!(
            shifted,
            vec![cue(0, 500, "overlap"), cue(1000, 1500, "inside")]
        );
    }
}
