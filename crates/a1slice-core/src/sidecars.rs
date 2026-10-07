//! Files written beside a video, plus settings and the recent list.
//!
//! JSON field names match the Electron app.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::preview::preview_cache_key;
use crate::types::{
    AppSettings, CaptionProject, ClipCrop, ClipSegment, ClipSegmentWithStatus, CropRatio, Ms,
    TranscriptSegment, VideoInspection, DEFAULT_CROP,
};

pub fn a1_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".a1slice")
}

pub fn video_stem(video_path: &Path) -> (PathBuf, String) {
    let dir = video_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let name = video_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let stem = name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(name)
        .to_string();
    (dir, stem)
}

pub fn transcript_json_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice.json"))
}

pub fn transcript_text_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}-transcript.txt"))
}

pub fn clips_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-clips.json"))
}

pub fn analysis_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-analysis.txt"))
}

pub fn framing_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-framing.json"))
}

pub fn captions_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-captions.json"))
}

pub fn recent_path() -> PathBuf {
    a1_dir().join("recent.json")
}

pub fn settings_path() -> PathBuf {
    a1_dir().join("settings.json")
}

pub fn models_dir() -> PathBuf {
    a1_dir().join("models")
}

pub fn model_path() -> PathBuf {
    models_dir().join("ggml-large-v3.bin")
}

pub fn preview_cache_dir() -> PathBuf {
    a1_dir().join("previews")
}

pub fn cached_preview_path(key: &str) -> PathBuf {
    preview_cache_dir().join(format!("{key}.mp4"))
}

fn read_json(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn read_transcript(video_path: &Path) -> Vec<TranscriptSegment> {
    let Some(value) = read_json(&transcript_json_path(video_path)) else {
        return Vec::new();
    };
    let Value::Array(items) = value else {
        return Vec::new();
    };
    items.iter().filter_map(parse_segment).collect()
}

fn parse_segment(value: &Value) -> Option<TranscriptSegment> {
    let obj = value.as_object()?;
    let start_ms = obj
        .get("startMs")?
        .as_i64()
        .or_else(|| obj.get("startMs")?.as_f64().map(|n| n as i64))?;
    let end_ms = obj
        .get("endMs")?
        .as_i64()
        .or_else(|| obj.get("endMs")?.as_f64().map(|n| n as i64))?;
    let text = obj.get("text")?.as_str()?.to_string();
    if end_ms <= start_ms {
        return None;
    }
    Some(TranscriptSegment {
        start_ms,
        end_ms,
        text,
    })
}

pub fn read_clips(video_path: &Path) -> (Vec<ClipSegment>, String) {
    let raw = fs::read_to_string(analysis_path(video_path)).unwrap_or_default();
    let Some(value) = read_json(&clips_path(video_path)) else {
        return (Vec::new(), raw);
    };
    (parse_clips_cache(&value), raw)
}

pub fn parse_clips_cache(value: &Value) -> Vec<ClipSegment> {
    let Value::Array(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let title = obj.get("title")?.as_str()?.to_string();
            let start_ms = num_i64(obj.get("startMs")?)?;
            let end_ms = num_i64(obj.get("endMs")?)?;
            let category = obj
                .get("category")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "related" => Some(crate::types::ClipCategory::Related),
                    "standalone" => Some(crate::types::ClipCategory::Standalone),
                    _ => None,
                });
            let topic = obj
                .get("topic")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let crop = obj.get("crop").and_then(parse_crop);
            let approved = obj.get("approved").and_then(|v| v.as_bool());
            Some(ClipSegment {
                title,
                start_ms,
                end_ms,
                category,
                topic,
                crop,
                approved,
            })
        })
        .collect()
}

fn num_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|n| n.round() as i64))
}

fn parse_crop(value: &Value) -> Option<ClipCrop> {
    let obj = value.as_object()?;
    let ratio = CropRatio::parse(obj.get("ratio")?.as_str()?)?;
    let cx = obj.get("cx")?.as_f64()?;
    let cy = obj.get("cy")?.as_f64()?;
    let zoom = obj.get("zoom")?.as_f64()?;
    if ![cx, cy, zoom].iter().all(|n| n.is_finite()) {
        return None;
    }
    Some(ClipCrop {
        ratio,
        cx: cx.clamp(0.0, 1.0),
        cy: cy.clamp(0.0, 1.0),
        zoom: zoom.clamp(0.0, 1.0),
    })
}

pub fn to_stored_clip(clip: &ClipSegment) -> ClipSegment {
    ClipSegment {
        title: clip.title.clone(),
        start_ms: clip.start_ms,
        end_ms: clip.end_ms,
        category: clip.category,
        topic: clip.topic.clone().filter(|t| !t.is_empty()),
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

pub fn read_framing(video_path: &Path) -> std::collections::BTreeMap<CropRatio, ClipCrop> {
    let mut framing = std::collections::BTreeMap::new();
    let Some(Value::Object(obj)) = read_json(&framing_path(video_path)) else {
        return framing;
    };
    for (key, raw) in obj {
        let Some(ratio) = CropRatio::parse(&key) else {
            continue;
        };
        let Some(crop) = parse_crop(&raw) else {
            continue;
        };
        if crop.ratio != ratio {
            continue;
        }
        framing.insert(ratio, crop);
    }
    framing
}

pub fn read_captions(video_path: &Path) -> Option<CaptionProject> {
    serde_json::from_value(read_json(&captions_path(video_path))?).ok()
}

pub fn inspect_video(video_path: &Path) -> VideoInspection {
    if !video_path.is_file() {
        return VideoInspection::default();
    }
    let segments = read_transcript(video_path);
    let (clips, _) = read_clips(video_path);
    let framing = read_framing(video_path);
    let captions = read_captions(video_path);
    VideoInspection {
        has_transcript: !segments.is_empty(),
        segment_count: segments.len(),
        has_clips: !clips.is_empty(),
        clip_count: clips.len(),
        has_framing: !framing.is_empty(),
        has_captions: captions.is_some(),
    }
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

pub fn format_srt_time(ms: Ms) -> String {
    let ms = ms.max(0);
    let total_seconds = ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
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

pub fn shift_subtitles(
    subtitles: &[TranscriptSegment],
    start_ms: Ms,
    end_ms: Ms,
) -> Vec<TranscriptSegment> {
    subtitles
        .iter()
        .filter(|segment| segment.end_ms > start_ms && segment.start_ms < end_ms)
        .map(|segment| TranscriptSegment {
            start_ms: (segment.start_ms - start_ms).max(0),
            end_ms: (end_ms - start_ms).min(segment.end_ms - start_ms),
            text: segment.text.clone(),
        })
        .collect()
}

pub fn preview_key_for(video_path: &Path, file_start_ms: i64, file_end_ms: i64) -> String {
    let meta = fs::metadata(video_path).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = meta
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0);
    preview_cache_key(
        &video_path.display().to_string(),
        size,
        mtime,
        file_start_ms,
        file_end_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_times_and_cues() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
        assert_eq!(format_srt_time(500), "00:00:00,500");
        assert_eq!(format_srt_time(5123), "00:00:05,123");
        assert_eq!(format_srt_time(90000), "00:01:30,000");
        assert_eq!(format_srt_time(3661001), "01:01:01,001");
        assert_eq!(generate_srt(&[]), "");
        let one = generate_srt(&[TranscriptSegment {
            start_ms: 0,
            end_ms: 2000,
            text: "Hello world".into(),
        }]);
        assert_eq!(one, "1\n00:00:00,000 --> 00:00:02,000\nHello world\n");
        let two = generate_srt(&[
            TranscriptSegment {
                start_ms: 0,
                end_ms: 2000,
                text: "First".into(),
            },
            TranscriptSegment {
                start_ms: 2000,
                end_ms: 5000,
                text: "Second".into(),
            },
        ]);
        assert_eq!(
            two,
            "1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n"
        );
    }

    #[test]
    fn older_clip_file_is_kept() {
        let clips =
            parse_clips_cache(&serde_json::json!([{ "title": "Old", "startMs": 10, "endMs": 20 }]));
        assert_eq!(clips[0].title, "Old");
        assert!(clips[0].approved.is_none());
        let status = with_clip_status(&clips);
        assert!(status[0].approved);
        assert_eq!(status[0].crop.ratio, CropRatio::Original);
    }

    #[test]
    fn dropped_clip_and_crop_round_trip() {
        let clips = parse_clips_cache(&serde_json::json!([{
            "title": "Hook",
            "startMs": 0,
            "endMs": 4000,
            "approved": false,
            "crop": { "ratio": "9:16", "cx": 0.4, "cy": 0.5, "zoom": 0.2 }
        }]));
        assert_eq!(clips[0].approved, Some(false));
        assert_eq!(clips[0].crop.unwrap().ratio, CropRatio::R9x16);
    }

    #[test]
    fn recent_list_caps_at_eight_and_moves_the_latest_first() {
        let paths: Vec<String> = (0..8).map(|i| format!("/v{i}.mp4")).collect();
        let next = remember_recent(&paths, "/v3.mp4", 8);
        assert_eq!(next[0], "/v3.mp4");
        assert_eq!(next.len(), 8);
        assert!(!next[1..].contains(&"/v3.mp4".to_string()));
    }

    #[test]
    fn unknown_settings_key_survives() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());
        let mut settings = AppSettings::default();
        settings.user_hint = "keep the demo".into();
        fs::create_dir_all(a1_dir()).unwrap();
        fs::write(settings_path(), r#"{"extra":"stay","provider":"claude"}"#).unwrap();
        save_settings(&settings).unwrap();
        let saved: Value =
            serde_json::from_str(&fs::read_to_string(settings_path()).unwrap()).unwrap();
        assert_eq!(saved["extra"], "stay");
        assert_eq!(saved["userHint"], "keep the demo");
        if let Some(prev) = prev {
            std::env::set_var("HOME", prev);
        }
    }
}
