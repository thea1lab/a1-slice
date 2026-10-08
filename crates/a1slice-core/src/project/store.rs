//! Read and write the sidecar files.

use crate::types::{
    CaptionProject, ClipCrop, ClipSegment, CropRatio, Ms, ProjectData, TranscriptSegment,
    VideoInspection,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::parse::{
    format_transcript_text, loose_segment, parse_captions, parse_clips_cache, parse_framing,
    remember_recent, to_stored_clip,
};
use super::paths::{
    a1slice_dir, analysis_path, captions_path, clips_path, framing_path, transcript_json_path,
    transcript_text_file, RECENT_LIMIT,
};

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
