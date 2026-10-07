//! Transcript, clip, framing, and caption files beside a video.
//!
//! Parsers match `src/shared/project.ts`. Reads and writes match `projectStore.ts`.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use serde_json::Value;

use crate::types::{
    CaptionProject, CaptionSource, ClipCategory, ClipCrop, ClipSegment, ClipSegmentWithStatus,
    CropRatio, Ms, ProjectData, TranscriptSegment, VideoInspection, DEFAULT_CROP,
};

const RECENT_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStem {
    pub dir: PathBuf,
    pub stem: String,
}

impl VideoStem {
    pub fn join(&self, file_name: impl AsRef<Path>) -> PathBuf {
        if self.dir.as_os_str() == "." {
            PathBuf::from(file_name.as_ref())
        } else {
            self.dir.join(file_name)
        }
    }
}

pub fn a1slice_dir() -> PathBuf {
    home_dir().join(".a1slice")
}

fn home_dir() -> PathBuf {
    for key in ["HOME", "USERPROFILE"] {
        if let Some(home) = std::env::var_os(key) {
            if !home.is_empty() {
                return PathBuf::from(home);
            }
        }
    }
    PathBuf::from(".")
}

pub fn video_stem(video_path: &Path) -> VideoStem {
    let name = video_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = strip_one_extension(&name).to_string();
    let dir = video_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    VideoStem { dir, stem }
}

fn strip_one_extension(base: &str) -> &str {
    match base.rfind('.') {
        Some(dot) if dot + 1 < base.len() => &base[..dot],
        _ => base,
    }
}

pub fn transcript_json_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice.json", located.stem))
}

pub fn clips_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-clips.json", located.stem))
}

pub fn analysis_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-analysis.txt", located.stem))
}

pub fn framing_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-framing.json", located.stem))
}

pub fn captions_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-captions.json", located.stem))
}

fn transcript_text_file(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}-transcript.txt", located.stem))
}

pub fn transcript_text_path(video_path: &str) -> String {
    let base = video_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(video_path);
    let name = format!("{}-transcript.txt", strip_one_extension(base));
    match video_path.rfind(['/', '\\']) {
        Some(slash) => format!("{}{name}", &video_path[..=slash]),
        None => name,
    }
}

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

fn loose_segment(value: &Value) -> Option<TranscriptSegment> {
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

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_json(path: &Path, value: &(impl serde::Serialize + ?Sized)) -> io::Result<()> {
    let bytes =
        serde_json::to_vec(value).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, bytes)
}

pub fn read_transcript(video_path: &Path) -> Vec<TranscriptSegment> {
    let Some(Value::Array(items)) = read_json(&transcript_json_path(video_path)) else {
        return Vec::new();
    };
    items.iter().filter_map(loose_segment).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipRead {
    pub clips: Vec<ClipSegment>,
    pub raw_response: String,
}

pub fn read_clips(video_path: &Path) -> ClipRead {
    let raw_response = fs::read_to_string(analysis_path(video_path)).unwrap_or_default();
    let clips = match read_json(&clips_path(video_path)) {
        Some(value) => parse_clips_cache(&value),
        None => Vec::new(),
    };
    ClipRead {
        clips,
        raw_response,
    }
}

pub fn read_framing(video_path: &Path) -> BTreeMap<CropRatio, ClipCrop> {
    read_json(&framing_path(video_path))
        .map(|value| parse_framing(&value))
        .unwrap_or_default()
}

pub fn read_captions(video_path: &Path) -> Option<CaptionProject> {
    parse_captions(&read_json(&captions_path(video_path))?)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFiles {
    pub segments: Vec<TranscriptSegment>,
    pub clips: Vec<ClipSegment>,
    pub raw_response: String,
    pub framing: BTreeMap<CropRatio, ClipCrop>,
    pub captions: Option<CaptionProject>,
}

impl ProjectFiles {
    pub fn with_duration(self, duration_ms: Ms) -> ProjectData {
        ProjectData {
            segments: self.segments,
            clips: self.clips,
            raw_response: self.raw_response,
            framing: self.framing,
            captions: self.captions,
            duration_ms,
        }
    }
}

pub fn read_project_files(video_path: &Path) -> ProjectFiles {
    let ClipRead {
        clips,
        raw_response,
    } = read_clips(video_path);
    ProjectFiles {
        segments: read_transcript(video_path),
        clips,
        raw_response,
        framing: read_framing(video_path),
        captions: read_captions(video_path),
    }
}

pub fn inspect_video(video_path: &Path) -> VideoInspection {
    if video_path.as_os_str().is_empty() || !video_path.exists() {
        return VideoInspection::default();
    }
    let segments = read_transcript(video_path);
    let clips = read_clips(video_path);
    let framing = read_framing(video_path);
    let captions = read_captions(video_path);
    VideoInspection {
        has_transcript: !segments.is_empty(),
        segment_count: segments.len(),
        has_clips: !clips.clips.is_empty(),
        clip_count: clips.clips.len(),
        has_framing: !framing.is_empty(),
        has_captions: captions.is_some(),
    }
}

pub fn write_transcript(video_path: &Path, segments: &[TranscriptSegment]) -> io::Result<PathBuf> {
    write_json(&transcript_json_path(video_path), segments)?;
    let text_path = transcript_text_file(video_path);
    let text = format_transcript_text(segments);
    let body = if text.is_empty() {
        String::new()
    } else {
        format!("{text}\n")
    };
    fs::write(&text_path, body)?;
    Ok(text_path)
}

pub fn write_clips(
    video_path: &Path,
    clips: &[ClipSegment],
    raw_response: Option<&str>,
) -> io::Result<()> {
    let stored: Vec<ClipSegment> = clips.iter().map(to_stored_clip).collect();
    write_json(&clips_path(video_path), &stored)?;
    if let Some(raw) = raw_response {
        fs::write(analysis_path(video_path), raw)?;
    }
    Ok(())
}

pub fn write_framing(video_path: &Path, framing: &BTreeMap<CropRatio, ClipCrop>) -> io::Result<()> {
    let mut object = serde_json::Map::new();
    for (ratio, crop) in framing {
        let value = serde_json::to_value(crop)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        object.insert(ratio.as_str().to_string(), value);
    }
    write_json(&framing_path(video_path), &Value::Object(object))
}

pub fn write_captions(video_path: &Path, captions: &CaptionProject) -> io::Result<()> {
    write_json(&captions_path(video_path), captions)
}

pub fn recent_path_in(a1_dir: &Path) -> PathBuf {
    a1_dir.join("recent.json")
}

pub fn recent_path() -> PathBuf {
    recent_path_in(&a1slice_dir())
}

pub fn read_recent_in(a1_dir: &Path) -> Vec<String> {
    let Some(Value::Array(items)) = read_json(&recent_path_in(a1_dir)) else {
        return Vec::new();
    };
    items
        .into_iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect()
}

pub fn read_recent() -> Vec<String> {
    read_recent_in(&a1slice_dir())
}

pub fn write_remembered_in(a1_dir: &Path, video_path: &str) -> io::Result<Vec<String>> {
    fs::create_dir_all(a1_dir)?;
    let next = remember_recent(&read_recent_in(a1_dir), video_path, RECENT_LIMIT);
    let text = serde_json::to_string_pretty(&next)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(recent_path_in(a1_dir), text)?;
    Ok(next)
}

pub fn write_remembered(video_path: &str) -> io::Result<Vec<String>> {
    write_remembered_in(&a1slice_dir(), video_path)
}

pub fn preview_cache_dir_in(a1_dir: &Path) -> PathBuf {
    a1_dir.join("previews")
}

pub fn preview_cache_dir() -> PathBuf {
    preview_cache_dir_in(&a1slice_dir())
}

pub fn cached_preview_path_in(a1_dir: &Path, key: &str) -> PathBuf {
    preview_cache_dir_in(a1_dir).join(format!("{key}.mp4"))
}

pub fn cached_preview_path(key: &str) -> PathBuf {
    cached_preview_path_in(&a1slice_dir(), key)
}

pub fn is_cached_preview_in(a1_dir: &Path, preview_path: impl AsRef<Path>) -> bool {
    let root = lexical_absolute(&preview_cache_dir_in(a1_dir));
    let file = lexical_absolute(preview_path.as_ref());
    file == root || file.starts_with(&root)
}

pub fn is_cached_preview(preview_path: impl AsRef<Path>) -> bool {
    is_cached_preview_in(&a1slice_dir(), preview_path)
}

fn lexical_absolute(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CaptionColor, CaptionFont, CaptionLook, CaptionPosition, CaptionSize, CaptionSource,
        CaptionStyle,
    };
    use serde_json::json;

    fn segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 2000,
                text: "Hello world".into(),
            },
            TranscriptSegment {
                start_ms: 2000,
                end_ms: 5000,
                text: "Second".into(),
            },
        ]
    }

    fn style(
        color: CaptionColor,
        position: CaptionPosition,
        size: CaptionSize,
        font: CaptionFont,
    ) -> CaptionStyle {
        CaptionStyle {
            color,
            custom_color: None,
            position,
            size,
            font_size: None,
            font,
        }
    }

    #[test]
    fn keeps_crop_and_a_dropped_clip() {
        let clips = parse_clips_cache(&json!([{
            "title": "Hook",
            "startMs": 0,
            "endMs": 4000,
            "approved": false,
            "crop": { "ratio": "9:16", "cx": 0.4, "cy": 0.5, "zoom": 0.2 }
        }]));
        assert_eq!(clips[0].approved, Some(false));
        assert_eq!(
            clips[0].crop,
            Some(ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.4,
                cy: 0.5,
                zoom: 0.2
            })
        );
    }

    #[test]
    fn loads_an_older_file_that_only_has_title_and_times() {
        let clips = parse_clips_cache(&json!([{ "title": "Old", "startMs": 10, "endMs": 20 }]));
        assert_eq!(
            clips,
            vec![ClipSegment {
                title: "Old".into(),
                start_ms: 10,
                end_ms: 20,
                category: None,
                topic: None,
                crop: None,
                approved: None,
            }]
        );
        let with_status = with_clip_status(&clips);
        assert_eq!(with_status[0].id, "0");
        assert!(with_status[0].approved);
        assert_eq!(with_status[0].crop, DEFAULT_CROP);
    }

    #[test]
    fn drops_the_ui_id_and_records_approval() {
        let stored = to_stored_clip(&ClipSegment {
            title: "A".into(),
            start_ms: 1,
            end_ms: 2,
            category: None,
            topic: None,
            crop: None,
            approved: Some(true),
        });
        assert_eq!(
            stored,
            ClipSegment {
                title: "A".into(),
                start_ms: 1,
                end_ms: 2,
                category: None,
                topic: None,
                crop: None,
                approved: Some(true),
            }
        );
    }

    #[test]
    fn returns_the_saved_story_frame() {
        let framing = parse_framing(&json!({
            "9:16": { "ratio": "9:16", "cx": 0.42, "cy": 0.5, "zoom": 0.1 },
            "nope": { "ratio": "9:16", "cx": 0.1, "cy": 0.1, "zoom": 0 }
        }));
        assert_eq!(
            framing.get(&CropRatio::R9x16).copied(),
            Some(ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.42,
                cy: 0.5,
                zoom: 0.1
            })
        );
        assert!(framing.get(&CropRatio::R16x9).is_none());
    }

    #[test]
    fn parses_an_srt_back_into_segments() {
        let srt = [
            "1",
            "00:00:00,000 --> 00:00:02,000",
            "Hello world",
            "",
            "2",
            "00:00:02,000 --> 00:00:05,000",
            "Second",
            "",
        ]
        .join("\n");
        assert_eq!(parse_srt(&srt), segments());
    }

    #[test]
    fn reads_a_saved_caption_project() {
        assert_eq!(
            parse_captions(&json!({
                "source": "manual",
                "look": "burn",
                "style": { "color": "yellow", "position": "top", "size": "medium", "font": "serif" },
                "cues": segments()
            })),
            Some(CaptionProject {
                source: CaptionSource::Manual,
                look: CaptionLook::Burn,
                style: style(
                    CaptionColor::Yellow,
                    CaptionPosition::Top,
                    CaptionSize::Medium,
                    CaptionFont::Serif
                ),
                cues: segments(),
                file_path: None,
            })
        );
    }

    #[test]
    fn reads_a_chosen_caption_file_only_for_pasted_words() {
        assert_eq!(
            parse_captions(&json!({
                "source": "manual",
                "look": "burn",
                "cues": segments(),
                "filePath": "/videos/other.srt"
            }))
            .and_then(|project| project.file_path),
            Some("/videos/other.srt".into())
        );
        assert_eq!(
            parse_captions(&json!({
                "source": "transcript",
                "look": "srt",
                "cues": [],
                "filePath": "/videos/other.srt"
            }))
            .and_then(|project| project.file_path),
            None
        );
    }

    #[test]
    fn reads_a_transcript_file_and_prefers_a_subtitle_file() {
        assert_eq!(
            parse_transcript_lines("0:00  Hello world\n0:02  Second\n"),
            vec![
                TranscriptSegment {
                    start_ms: 0,
                    end_ms: 2000,
                    text: "Hello world".into()
                },
                TranscriptSegment {
                    start_ms: 2000,
                    end_ms: 4000,
                    text: "Second".into()
                },
            ]
        );
        assert_eq!(
            parse_transcript_lines("1:02:03  Hello")
                .into_iter()
                .map(|line| line.start_ms)
                .collect::<Vec<_>>(),
            vec![3_723_000]
        );
        let srt = ["1", "00:00:01,000 --> 00:00:03,000", "Hello", ""].join("\n");
        assert_eq!(
            parse_caption_document(&srt),
            vec![TranscriptSegment {
                start_ms: 1000,
                end_ms: 3000,
                text: "Hello".into()
            }]
        );
        assert_eq!(
            parse_caption_document("0:12  Hello there"),
            vec![TranscriptSegment {
                start_ms: 12000,
                end_ms: 14000,
                text: "Hello there".into()
            }]
        );
    }

    #[test]
    fn keeps_an_older_burn_preset_as_a_size() {
        assert_eq!(
            parse_captions(
                &json!({ "source": "manual", "look": "burn-large", "cues": segments() })
            )
            .unwrap()
            .style
            .size,
            CaptionSize::Large
        );
        assert_eq!(
            parse_captions(&json!({ "source": "manual", "look": "burn-small", "cues": [] }))
                .unwrap()
                .style,
            style(
                CaptionColor::White,
                CaptionPosition::Bottom,
                CaptionSize::Small,
                CaptionFont::Sans
            )
        );
        assert_eq!(
            parse_captions(&json!({ "source": "transcript", "look": "srt", "cues": [] }))
                .unwrap()
                .look,
            CaptionLook::Srt
        );
    }

    #[test]
    fn writes_a_readable_transcript() {
        let text = format_transcript_text(&segments());
        assert!(text.contains("0:00  Hello world"));
        assert!(text.contains("0:02  Second"));
        assert!(format_transcript_text(&[TranscriptSegment {
            start_ms: 3_723_000,
            end_ms: 3_724_000,
            text: "Hello".into(),
        }])
        .contains("1:02:03  Hello"));
    }

    #[test]
    fn names_the_readable_transcript_beside_the_video() {
        assert_eq!(
            transcript_text_path("/videos/talk.mp4"),
            "/videos/talk-transcript.txt"
        );
        assert_eq!(
            transcript_text_path(r"C:\videos\talk.mov"),
            r"C:\videos\talk-transcript.txt"
        );
        assert_eq!(
            transcript_text_path("/videos/my.clip.mkv"),
            "/videos/my.clip-transcript.txt"
        );
        assert_eq!(transcript_text_path("talk.webm"), "talk-transcript.txt");
    }

    #[test]
    fn puts_the_newest_path_first_and_caps_the_list() {
        let paths = ["a", "b", "c", "d", "e", "f", "g", "h"];
        assert_eq!(
            remember_recent(&paths, "c", 8),
            vec!["c", "a", "b", "d", "e", "f", "g", "h"]
        );
        assert_eq!(remember_recent(&paths, "new", 3), vec!["new", "a", "b"]);
    }

    #[test]
    fn preview_cache_key_stays_stable_for_the_same_file_and_range() {
        let key = preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0);
        assert_eq!(key, "ea259571");
        assert_eq!(preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0), key);
        assert_eq!(preview_cache_key("/v.mp4", 10, 20.4, 0.0, 5000.0), key);
    }

    #[test]
    fn preview_cache_key_changes_when_the_range_or_the_file_changes() {
        let key = preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0);
        assert_ne!(preview_cache_key("/v.mp4", 10, 20.0, 1000.0, 5000.0), key);
        assert_ne!(preview_cache_key("/v.mp4", 10, 21.0, 0.0, 5000.0), key);
    }

    #[test]
    fn writes_sidecar_files_and_reads_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("talk.mp4");
        fs::write(&video, b"video").unwrap();
        let text_path = write_transcript(&video, &segments()).unwrap();
        assert_eq!(text_path, dir.path().join("talk-transcript.txt"));
        let readable = fs::read_to_string(&text_path).unwrap();
        assert!(readable.ends_with('\n'));
        assert!(readable.contains("0:00  Hello world"));
        assert_eq!(read_transcript(&video), segments());

        let clip = ClipSegment {
            title: "Hook".into(),
            start_ms: 0,
            end_ms: 4000,
            category: Some(ClipCategory::Standalone),
            topic: Some("open".into()),
            crop: None,
            approved: Some(false),
        };
        write_clips(&video, &[clip.clone()], Some("raw answer")).unwrap();
        let loaded = read_clips(&video);
        assert_eq!(loaded.raw_response, "raw answer");
        assert_eq!(loaded.clips[0].approved, Some(false));
        assert_eq!(loaded.clips[0].topic.as_deref(), Some("open"));

        let mut framing = BTreeMap::new();
        framing.insert(
            CropRatio::R9x16,
            ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.4,
                cy: 0.5,
                zoom: 0.2,
            },
        );
        write_framing(&video, &framing).unwrap();
        assert_eq!(read_framing(&video), framing);

        let inspection = inspect_video(&video);
        assert!(inspection.has_transcript);
        assert_eq!(inspection.segment_count, 2);
        assert!(inspection.has_clips);
        assert!(inspection.has_framing);
        assert!(!inspection.has_captions);
        assert!(!inspect_video(Path::new("/missing/nope.mp4")).has_transcript);
    }

    #[test]
    fn remembers_paths_inside_the_given_directory() {
        let dir = tempfile::tempdir().unwrap();
        let a1 = dir.path().join("a1");
        let first = write_remembered_in(&a1, "/v/a.mp4").unwrap();
        assert_eq!(first, vec!["/v/a.mp4".to_string()]);
        let second = write_remembered_in(&a1, "/v/b.mp4").unwrap();
        assert_eq!(second, vec!["/v/b.mp4".to_string(), "/v/a.mp4".to_string()]);
        assert_eq!(read_recent_in(&a1), second);
        let again = write_remembered_in(&a1, "/v/a.mp4").unwrap();
        assert_eq!(again[0], "/v/a.mp4");
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn cached_preview_paths_stay_inside_the_cache_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = preview_cache_dir_in(dir.path());
        let file = cached_preview_path_in(dir.path(), "ea259571");
        assert!(is_cached_preview_in(dir.path(), &file));
        assert!(is_cached_preview_in(dir.path(), &root));
        assert!(!is_cached_preview_in(
            dir.path(),
            dir.path().join("previews-evil").join("x.mp4")
        ));
    }
}
