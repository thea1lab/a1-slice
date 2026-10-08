//! Turn the last JSON array an agent prints into clips.

use crate::types::{ClipCategory, ClipSegment, Ms, TranscriptSegment};
use serde_json::{Map, Value};
use thiserror::Error;

pub(super) fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub fn ms_to_timecode(ms: Ms) -> String {
    let total_seconds = ms.div_euclid(1000);
    let hours = total_seconds.div_euclid(3600);
    let minutes = total_seconds.rem_euclid(3600) / 60;
    let seconds = total_seconds.rem_euclid(60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub fn format_transcript_for_llm(segments: &[TranscriptSegment], start_index: usize) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "[#{} {} -> {} | {} -> {}] {}",
                start_index + i,
                ms_to_timecode(seg.start_ms),
                ms_to_timecode(seg.end_ms),
                seg.start_ms,
                seg.end_ms,
                seg.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("No JSON array found in LLM response")]
pub struct NoJsonArray;

pub fn extract_json_array(text: &str) -> Result<Vec<Value>, NoJsonArray> {
    let fenced = first_fence_body(text);
    let mut sources: Vec<&str> = Vec::new();
    if let Some(body) = fenced.as_deref() {
        sources.push(body);
    }
    sources.push(text);
    for src in sources {
        let arrays = scan_arrays(src);
        if let Some(last) = arrays.last() {
            return Ok(last.clone());
        }
    }
    Err(NoJsonArray)
}

fn first_fence_body(text: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find("```") {
        let start = search_from + rel;
        let mut j = start + 3;
        let rest = &text[j..];
        if rest.len() >= 4 && rest.is_char_boundary(4) && rest[..4].eq_ignore_ascii_case("json") {
            j += 4;
        }
        while j < text.len() && text.as_bytes()[j].is_ascii_whitespace() {
            j += 1;
        }
        if let Some(end_rel) = text[j..].find("```") {
            return Some(text[j..j + end_rel].trim().to_string());
        }
        search_from = start + 3;
    }
    None
}

fn scan_arrays(src: &str) -> Vec<Vec<Value>> {
    let mut arrays = Vec::new();
    let mut i = 0;
    while i < src.len() {
        let ch = src[i..].chars().next().unwrap();
        if ch == '[' || ch == '{' {
            if let Some(slice) = extract_balanced(src, i) {
                if let Ok(parsed) = serde_json::from_str::<Value>(slice) {
                    if let Some(arr) = unwrap_json_array(&parsed) {
                        arrays.push(arr);
                        i += slice.len();
                        continue;
                    }
                }
            }
        }
        i += ch.len_utf8();
    }
    arrays
}

fn extract_balanced(text: &str, start: usize) -> Option<&str> {
    let open = text[start..].chars().next()?;
    let close = if open == '[' { ']' } else { '}' };
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;
    while i < text.len() {
        let ch = text[i..].chars().next()?;
        let ch_len = ch.len_utf8();
        if in_string {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += ch_len;
            continue;
        }
        if ch == '"' {
            in_string = true;
            i += ch_len;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(&text[start..i + ch_len]);
            }
        }
        i += ch_len;
    }
    None
}

fn unwrap_json_array(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => Some(items.clone()),
        Value::Object(obj) => {
            for key in [
                "clips",
                "items",
                "hooks",
                "candidates",
                "segments",
                "topics",
            ] {
                if let Some(Value::Array(items)) = obj.get(key) {
                    return Some(items.clone());
                }
            }
            let arrays: Vec<&Vec<Value>> = obj.values().filter_map(Value::as_array).collect();
            if arrays.len() == 1 {
                Some(arrays[0].clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn coerce_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64().filter(|n| n.is_finite()),
        Value::String(text) => parse_js_number(text),
        _ => None,
    }
}

fn parse_js_number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn coerce_ms(value: &Value) -> Option<Ms> {
    match value {
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                return Some(integer);
            }
            if let Some(integer) = number.as_u64() {
                return i64::try_from(integer).ok();
            }
            number.as_f64().filter(|n| n.is_finite()).map(|n| n as Ms)
        }
        Value::String(text) => parse_js_number(text).map(|n| n as Ms),
        _ => None,
    }
}

fn array_index(n: f64, len: usize) -> Option<usize> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return None;
    }
    let index = n as usize;
    if index < len {
        Some(index)
    } else {
        None
    }
}

fn clip_from_parsed_item(
    item: &Value,
    segments: Option<&[TranscriptSegment]>,
) -> Option<ClipSegment> {
    let obj = item.as_object()?;
    let title = match obj.get("title").and_then(Value::as_str) {
        Some(title) if !title.trim().is_empty() => title.to_string(),
        _ => "Clip".to_string(),
    };
    let (mut start_ms, mut end_ms) = resolved_bounds(obj, segments)?;
    if end_ms < start_ms {
        std::mem::swap(&mut start_ms, &mut end_ms);
    }
    let category = match obj.get("category").and_then(Value::as_str) {
        Some("related") => Some(ClipCategory::Related),
        Some("standalone") => Some(ClipCategory::Standalone),
        _ => None,
    };
    Some(ClipSegment {
        title,
        start_ms,
        end_ms,
        category,
        topic: None,
        crop: None,
        approved: None,
    })
}

fn resolved_bounds(
    obj: &Map<String, Value>,
    segments: Option<&[TranscriptSegment]>,
) -> Option<(Ms, Ms)> {
    if let Some(segments) = segments {
        if let (Some(start_id), Some(end_id)) = (
            obj.get("start_id").and_then(coerce_f64),
            obj.get("end_id").and_then(coerce_f64),
        ) {
            if let (Some(start_index), Some(end_index)) = (
                array_index(start_id, segments.len()),
                array_index(end_id, segments.len()),
            ) {
                return Some((segments[start_index].start_ms, segments[end_index].end_ms));
            }
        }
    }
    let start_ms = obj.get("start_ms").and_then(coerce_ms)?;
    let end_ms = obj.get("end_ms").and_then(coerce_ms)?;
    Some((start_ms, end_ms))
}

pub fn parse_llm_response(
    text: &str,
    segments: Option<&[TranscriptSegment]>,
) -> Result<Vec<ClipSegment>, NoJsonArray> {
    let parsed = extract_json_array(text)?;
    Ok(parsed
        .iter()
        .filter_map(|item| clip_from_parsed_item(item, segments))
        .collect())
}

/// Parse printed clip JSON, step one millisecond into the chosen line, then snap.
pub fn clips_from_agent_text(
    text: &str,
    segments: &[TranscriptSegment],
) -> Result<Vec<ClipSegment>, NoJsonArray> {
    let parsed = parse_llm_response(text, Some(segments))?;
    Ok(parsed
        .into_iter()
        .filter_map(|mut clip| {
            // When two lines meet, the shared instant belongs to the earlier line.
            clip.start_ms += 1;
            let clip = crate::clips::refine_clip_bounds(clip, segments);
            if clip.end_ms > clip.start_ms {
                Some(clip)
            } else {
                None
            }
        })
        .collect())
}
