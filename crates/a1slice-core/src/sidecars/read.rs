//! Read the sidecars beside a video.

use crate::types::{
    CaptionProject, ClipCrop, ClipSegment, ClipSegmentWithStatus, CropRatio, TranscriptSegment,
    VideoInspection, DEFAULT_CROP,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::paths::{analysis_path, captions_path, clips_path, framing_path, transcript_json_path};

pub(super) fn read_json(path: &Path) -> Option<Value> {
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
