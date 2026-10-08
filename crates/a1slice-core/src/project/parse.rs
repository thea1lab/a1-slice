//! Parse transcript, clip, framing, caption, and subtitle text.

use crate::types::{
    CaptionProject, CaptionSource, ClipCategory, ClipCrop, ClipSegment, ClipSegmentWithStatus,
    CropRatio, Ms, TranscriptSegment, DEFAULT_CROP,
};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn parse_crop(value: &Value) -> Option<ClipCrop> {
    let item = value.as_object()?;
    let ratio = CropRatio::parse(item.get("ratio")?.as_str()?)?;
    let cx = finite_f64(item.get("cx")?)?;
    let cy = finite_f64(item.get("cy")?)?;
    let zoom = finite_f64(item.get("zoom")?)?;
    Some(ClipCrop {
        ratio,
        cx: cx.clamp(0.0, 1.0),
        cy: cy.clamp(0.0, 1.0),
        zoom: zoom.clamp(0.0, 1.0),
    })
}

fn finite_f64(value: &Value) -> Option<f64> {
    let number = value.as_f64()?;
    number.is_finite().then_some(number)
}

fn finite_ms(value: &Value) -> Option<Ms> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    if let Some(number) = value.as_u64() {
        return Ms::try_from(number).ok();
    }
    let number = finite_f64(value)?;
    if number < i64::MIN as f64 || number > i64::MAX as f64 {
        return None;
    }
    Some(js_round(number))
}

pub(super) fn loose_segment(value: &Value) -> Option<TranscriptSegment> {
    let row = value.as_object()?;
    Some(TranscriptSegment {
        start_ms: finite_ms(row.get("startMs")?)?,
        end_ms: finite_ms(row.get("endMs")?)?,
        text: row.get("text")?.as_str()?.to_string(),
    })
}

fn parse_segments(value: &Value) -> Vec<TranscriptSegment> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let segment = loose_segment(item)?;
            (segment.end_ms > segment.start_ms).then_some(segment)
        })
        .collect()
}

pub fn parse_clips_cache(value: &Value) -> Vec<ClipSegment> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    let mut clips = Vec::new();
    for item in items {
        let Some(row) = item.as_object() else {
            continue;
        };
        let Some(title) = row.get("title").and_then(Value::as_str) else {
            continue;
        };
        let (Some(start_ms), Some(end_ms)) = (
            row.get("startMs").and_then(finite_ms),
            row.get("endMs").and_then(finite_ms),
        ) else {
            continue;
        };
        let category = row
            .get("category")
            .and_then(Value::as_str)
            .and_then(|kind| match kind {
                "related" => Some(ClipCategory::Related),
                "standalone" => Some(ClipCategory::Standalone),
                _ => None,
            });
        let topic = row
            .get("topic")
            .and_then(Value::as_str)
            .filter(|topic| !topic.is_empty())
            .map(str::to_string);
        let approved = match row.get("approved") {
            Some(Value::Bool(approved)) => Some(*approved),
            _ => None,
        };
        clips.push(ClipSegment {
            title: title.to_string(),
            start_ms,
            end_ms,
            category,
            topic,
            crop: row.get("crop").and_then(parse_crop),
            approved,
        });
    }
    clips
}

pub fn to_stored_clip(clip: &ClipSegment) -> ClipSegment {
    ClipSegment {
        title: clip.title.clone(),
        start_ms: clip.start_ms,
        end_ms: clip.end_ms,
        category: clip.category,
        topic: clip.topic.clone().filter(|topic| !topic.is_empty()),
        crop: clip.crop,
        approved: Some(clip.approved != Some(false)),
    }
}

pub fn with_clip_status(clips: &[ClipSegment]) -> Vec<ClipSegmentWithStatus> {
    clips
        .iter()
        .enumerate()
        .map(|(index, clip)| ClipSegmentWithStatus {
            id: index.to_string(),
            title: clip.title.clone(),
            start_ms: clip.start_ms,
            end_ms: clip.end_ms,
            category: clip.category,
            topic: clip.topic.clone(),
            crop: clip.crop.unwrap_or(DEFAULT_CROP),
            approved: clip.approved != Some(false),
        })
        .collect()
}

pub fn parse_framing(value: &Value) -> BTreeMap<CropRatio, ClipCrop> {
    let mut framing = BTreeMap::new();
    let Some(rows) = value.as_object() else {
        return framing;
    };
    for (key, raw) in rows {
        let Some(ratio) = CropRatio::parse(key) else {
            continue;
        };
        let Some(crop) = parse_crop(raw) else {
            continue;
        };
        if crop.ratio != ratio {
            continue;
        }
        framing.insert(ratio, crop);
    }
    framing
}

pub fn parse_captions(value: &Value) -> Option<CaptionProject> {
    let row = value.as_object()?;
    let source = match row.get("source").and_then(Value::as_str) {
        Some("manual") => CaptionSource::Manual,
        Some("transcript") => CaptionSource::Transcript,
        _ => return None,
    };
    let null = Value::Null;
    let look_value = row.get("look").unwrap_or(&null);
    let look = crate::captions::coerce_caption_look(look_value)?;
    let style_value = row.get("style").unwrap_or(&null);
    let style = crate::captions::coerce_caption_style(look_value, style_value);
    let cues = row.get("cues").map(parse_segments).unwrap_or_default();
    let file_path = if source == CaptionSource::Manual {
        row.get("filePath")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_string)
    } else {
        None
    };
    Some(CaptionProject {
        source,
        look,
        style,
        cues,
        file_path,
    })
}

pub fn remember_recent<S: AsRef<str>>(paths: &[S], video_path: &str, max: usize) -> Vec<String> {
    if video_path.is_empty() {
        return paths
            .iter()
            .take(max)
            .map(|path| path.as_ref().to_string())
            .collect();
    }
    let mut next = Vec::new();
    next.push(video_path.to_string());
    if next.len() >= max {
        next.truncate(max);
        return next;
    }
    for path in paths {
        let path = path.as_ref();
        if path != video_path {
            next.push(path.to_string());
            if next.len() == max {
                break;
            }
        }
    }
    next
}

pub fn format_transcript_text(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|segment| {
            let total = (segment.start_ms / 1000).max(0);
            let hours = total / 3600;
            let minutes = (total % 3600) / 60;
            let seconds = total % 60;
            let stamp = if hours > 0 {
                format!("{hours}:{minutes:02}:{seconds:02}")
            } else {
                format!("{minutes}:{seconds:02}")
            };
            format!("{stamp}  {}", segment.text.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_transcript_lines(text: &str) -> Vec<TranscriptSegment> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let mut rows = Vec::new();
    for raw in text.split('\n') {
        let Some((start_ms, spoken)) = parse_transcript_line(raw.trim_end_matches('\r')) else {
            continue;
        };
        rows.push((start_ms, spoken));
    }
    rows.iter()
        .enumerate()
        .map(|(index, (start_ms, spoken))| {
            let end_ms = rows
                .get(index + 1)
                .map(|(next, _)| *next)
                .filter(|next| *next > *start_ms)
                .unwrap_or(start_ms + 2000);
            TranscriptSegment {
                start_ms: *start_ms,
                end_ms,
                text: spoken.clone(),
            }
        })
        .collect()
}

fn parse_transcript_line(raw: &str) -> Option<(Ms, String)> {
    let line = raw.trim();
    let (time, rest) = line.split_once(char::is_whitespace)?;
    let spoken = rest.trim();
    if spoken.is_empty() {
        return None;
    }
    Some((parse_transcript_clock(time)?, spoken.to_string()))
}

fn parse_transcript_clock(time: &str) -> Option<Ms> {
    let (hms, fraction) = match time.find(['.', ',']) {
        Some(index) => {
            let fraction = &time[index + 1..];
            if fraction.is_empty()
                || fraction.len() > 3
                || !fraction.bytes().all(|byte| byte.is_ascii_digit())
            {
                return None;
            }
            (&time[..index], fraction)
        }
        None => (time, ""),
    };
    let pieces: Vec<&str> = hms.split(':').collect();
    let (hours, minutes, seconds) = match pieces.as_slice() {
        [minutes, seconds] => (0, digits(minutes)?, two_digits(seconds)?),
        [hours, minutes, seconds] => (digits(hours)?, digits(minutes)?, two_digits(seconds)?),
        _ => return None,
    };
    let millis = if fraction.is_empty() {
        0
    } else {
        fraction_to_millis(fraction)
    };
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}

fn digits(value: &str) -> Option<i64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn two_digits(value: &str) -> Option<i64> {
    if value.len() != 2 {
        return None;
    }
    digits(value)
}

fn fraction_to_millis(fraction: &str) -> i64 {
    if fraction.len() >= 3 {
        fraction[..3].parse().unwrap_or(0)
    } else {
        let number: i64 = fraction.parse().unwrap_or(0);
        let scale = 10i64.pow((3 - fraction.len()) as u32);
        number * scale
    }
}

pub fn parse_caption_document(text: &str) -> Vec<TranscriptSegment> {
    let srt = parse_srt(text);
    if !srt.is_empty() {
        srt
    } else {
        parse_transcript_lines(text)
    }
}

pub fn parse_srt(text: &str) -> Vec<TranscriptSegment> {
    let mut segments = Vec::new();
    for lines in srt_blocks(text) {
        let Some(time_index) = lines.iter().position(|line| line.contains("-->")) else {
            continue;
        };
        let Some((start_ms, end_ms)) = parse_srt_range(lines[time_index]) else {
            continue;
        };
        let cue = lines[time_index + 1..].join("\n");
        let cue = cue.trim();
        if cue.is_empty() || end_ms <= start_ms {
            continue;
        }
        segments.push(TranscriptSegment {
            start_ms,
            end_ms,
            text: cue.to_string(),
        });
    }
    segments
}

fn srt_blocks(text: &str) -> Vec<Vec<&str>> {
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text).trim();
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in text.split('\n') {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

fn parse_srt_range(line: &str) -> Option<(Ms, Ms)> {
    let index = line.find("-->")?;
    let start_ms = find_srt_clock(&line[..index])?;
    let end_ms = find_srt_clock(&line[index + 3..])?;
    Some((start_ms, end_ms))
}

fn find_srt_clock(text: &str) -> Option<Ms> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            if let Some(ms) = srt_clock_at(text, index) {
                return Some(ms);
            }
        }
        index += 1;
    }
    None
}

fn srt_clock_at(text: &str, start: usize) -> Option<Ms> {
    let bytes = text.as_bytes();
    let mut index = start;
    let hours_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index == hours_start || bytes.get(index) != Some(&b':') {
        return None;
    }
    let hours: i64 = text[hours_start..index].parse().ok()?;
    index += 1;
    let minutes = exact_digits(text, &mut index)?;
    if bytes.get(index) != Some(&b':') {
        return None;
    }
    index += 1;
    let seconds = exact_digits(text, &mut index)?;
    if !matches!(bytes.get(index), Some(b',' | b'.')) {
        return None;
    }
    index += 1;
    let fraction_start = index;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    let fraction_len = index - fraction_start;
    if !(1..=3).contains(&fraction_len) {
        return None;
    }
    let millis = fraction_to_millis(&text[fraction_start..index]);
    Some(hours * 3_600_000 + minutes * 60_000 + seconds * 1_000 + millis)
}

fn exact_digits(text: &str, index: &mut usize) -> Option<i64> {
    let bytes = text.as_bytes();
    if *index + 2 > bytes.len()
        || !bytes[*index].is_ascii_digit()
        || !bytes[*index + 1].is_ascii_digit()
    {
        return None;
    }
    if bytes
        .get(*index + 2)
        .is_some_and(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let value = text[*index..*index + 2].parse().ok()?;
    *index += 2;
    Some(value)
}

/// FNV-1a 32-bit over UTF-16 code units, matching `charCodeAt` in `previewCacheKey`.
pub fn preview_cache_key(
    video_path: &str,
    size: i64,
    mtime_ms: f64,
    file_start_ms: f64,
    file_end_ms: f64,
) -> String {
    let raw = format!(
        "{video_path}\0{size}\0{}\0{}\0{}",
        js_round(mtime_ms),
        js_round(file_start_ms),
        js_round(file_end_ms)
    );
    format!("{:08x}", fnv1a_32(&raw))
}

fn fnv1a_32(text: &str) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for unit in text.encode_utf16() {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn js_round(number: f64) -> i64 {
    if !number.is_finite() {
        return 0;
    }
    let shifted = number + 0.5;
    if !shifted.is_finite() {
        return 0;
    }
    shifted.floor() as i64
}
