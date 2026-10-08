//! Write sidecars, the recent list, and settings.

use crate::types::{
    AppSettings, CaptionProject, ClipCrop, ClipSegment, CropRatio, TranscriptSegment,
};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use super::paths::{
    a1_dir, analysis_path, captions_path, clips_path, framing_path, recent_path, settings_path,
    transcript_json_path, transcript_text_path,
};
use super::read::{read_json, to_stored_clip};

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

pub fn write_transcript(
    video_path: &Path,
    segments: &[TranscriptSegment],
) -> std::io::Result<PathBuf> {
    fs::write(
        transcript_json_path(video_path),
        serde_json::to_vec(segments).unwrap_or_default(),
    )?;
    let text_path = transcript_text_path(video_path);
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
) -> std::io::Result<()> {
    let stored: Vec<_> = clips.iter().map(to_stored_clip).collect();
    fs::write(
        clips_path(video_path),
        serde_json::to_vec(&stored).unwrap_or_default(),
    )?;
    if let Some(raw) = raw_response {
        fs::write(analysis_path(video_path), raw)?;
    }
    Ok(())
}

pub fn write_framing(
    video_path: &Path,
    framing: &std::collections::BTreeMap<CropRatio, ClipCrop>,
) -> std::io::Result<()> {
    fs::write(
        framing_path(video_path),
        serde_json::to_vec(framing).unwrap_or_default(),
    )
}

pub fn write_captions(video_path: &Path, captions: &CaptionProject) -> std::io::Result<()> {
    fs::write(
        captions_path(video_path),
        serde_json::to_vec(captions).unwrap_or_default(),
    )
}

pub fn read_recent() -> Vec<String> {
    match read_json(&recent_path()) {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub fn remember_recent(paths: &[String], video_path: &str, max: usize) -> Vec<String> {
    if video_path.is_empty() {
        return paths.iter().take(max).cloned().collect();
    }
    let mut next = vec![video_path.to_string()];
    next.extend(paths.iter().filter(|p| p.as_str() != video_path).cloned());
    next.truncate(max);
    next
}

pub fn write_remembered(video_path: &str) -> std::io::Result<Vec<String>> {
    fs::create_dir_all(a1_dir())?;
    let next = remember_recent(&read_recent(), video_path, 8);
    fs::write(
        recent_path(),
        serde_json::to_string_pretty(&next).unwrap_or_else(|_| "[]".into()),
    )?;
    Ok(next)
}

pub fn load_settings() -> AppSettings {
    let Some(mut value) = read_json(&settings_path()) else {
        return AppSettings::default();
    };
    migrate_api_keys(&mut value);
    serde_json::from_value(value).unwrap_or_default()
}

fn migrate_api_keys(value: &mut Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let provider = obj
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("claude")
        .to_string();
    let api_key = obj
        .get("apiKey")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let keys = obj
        .entry("apiKeys")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(map) = keys.as_object_mut() else {
        return;
    };
    if map.contains_key(&provider) {
        return;
    }
    if !api_key.is_empty() {
        map.insert(provider, Value::String(api_key));
        return;
    }
    let legacy = map
        .iter()
        .find(|(id, _)| id.as_str() == provider || id.starts_with(&format!("{provider}:")))
        .map(|(_, value)| value.clone());
    if let Some(legacy) = legacy {
        map.insert(provider, legacy);
    }
}

pub fn save_settings(settings: &AppSettings) -> std::io::Result<()> {
    fs::create_dir_all(a1_dir())?;
    let path = settings_path();
    let mut root = read_json(&path).unwrap_or_else(|| Value::Object(Map::new()));
    if !root.is_object() {
        root = Value::Object(Map::new());
    }
    let incoming = serde_json::to_value(settings).unwrap_or(Value::Null);
    if let (Some(root_obj), Some(incoming_obj)) = (root.as_object_mut(), incoming.as_object()) {
        for (key, value) in incoming_obj {
            root_obj.insert(key.clone(), value.clone());
        }
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".into()),
    )
}
