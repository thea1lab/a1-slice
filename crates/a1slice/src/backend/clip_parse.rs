//! Read the last clip list an agent prints.

use a1slice_core::clips::refine_clip_bounds;
use a1slice_core::types::{ClipSegment, TranscriptSegment};

pub(super) fn clips_from_text(
    text: &str,
    segments: &[TranscriptSegment],
) -> Result<Vec<ClipSegment>, String> {
    let arrays = extract_arrays(text);
    if arrays.is_empty() {
        return Err("The agent did not return a clip list.".into());
    }
    let mut found = Vec::new();
    for array in arrays {
        let clips = clips_from_array(&array, segments);
        if !clips.is_empty() {
            found = clips;
        }
    }
    Ok(found)
}

fn clips_from_array(
    array: &[serde_json::Value],
    segments: &[TranscriptSegment],
) -> Vec<ClipSegment> {
    let mut clips = Vec::new();
    for item in array {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Clip")
            .to_string();
        let start_id = obj
            .get("start_id")
            .or_else(|| obj.get("startId"))
            .and_then(json_index);
        let end_id = obj
            .get("end_id")
            .or_else(|| obj.get("endId"))
            .and_then(json_index);
        let (start_ms, end_ms) = if let (Some(start_id), Some(end_id)) = (start_id, end_id) {
            match (segments.get(start_id), segments.get(end_id)) {
                (Some(start), Some(end)) => (start.start_ms, end.end_ms),
                _ => continue,
            }
        } else {
            let Some(start_ms) = obj
                .get("start_ms")
                .or_else(|| obj.get("startMs"))
                .and_then(json_ms)
            else {
                continue;
            };
            let Some(end_ms) = obj
                .get("end_ms")
                .or_else(|| obj.get("endMs"))
                .and_then(json_ms)
            else {
                continue;
            };
            (start_ms, end_ms)
        };
        let (start_ms, end_ms) = if end_ms < start_ms {
            (end_ms, start_ms)
        } else {
            (start_ms, end_ms)
        };
        let category = match obj.get("category").and_then(|v| v.as_str()) {
            Some("related") => Some(a1slice_core::types::ClipCategory::Related),
            Some("standalone") => Some(a1slice_core::types::ClipCategory::Standalone),
            _ => None,
        };
        let clip = ClipSegment {
            title,
            start_ms: start_ms + 1,
            end_ms,
            category,
            topic: None,
            crop: None,
            approved: Some(true),
        };
        let refined = refine_clip_bounds(clip, segments);
        if refined.end_ms > refined.start_ms {
            clips.push(refined);
        }
    }
    clips
}

fn json_index(value: &serde_json::Value) -> Option<usize> {
    json_ms(value).filter(|n| *n >= 0).map(|n| n as usize)
}

fn json_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    if let Some(n) = value.as_u64() {
        return i64::try_from(n).ok();
    }
    if let Some(n) = value.as_f64() {
        if n.is_finite() {
            return Some(n.round() as i64);
        }
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(n) = text.parse::<i64>() {
        return Some(n);
    }
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(|n| n.round() as i64)
}

fn extract_arrays(text: &str) -> Vec<Vec<serde_json::Value>> {
    let fenced = text.split("```").nth(1).map(|block| {
        block
            .trim_start_matches("json")
            .trim_start_matches("JSON")
            .trim()
    });
    let owned;
    let sources: Vec<&str> = if let Some(block) = fenced {
        owned = block.to_string();
        vec![owned.as_str(), text]
    } else {
        vec![text]
    };
    for src in sources {
        let found = arrays_in(src);
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

fn arrays_in(src: &str) -> Vec<Vec<serde_json::Value>> {
    let bytes = src.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' || bytes[i] == b'{' {
            if let Some(slice) = balanced(src, i) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(slice) {
                    if let Some(array) = unwrap_array(&value) {
                        found.push(array);
                        i += slice.len();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    found
}

fn balanced(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let open = bytes.get(start).copied()?;
    let close = if open == b'[' { b']' } else { b'}' };
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if ch == b'\\' {
                escape = true;
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(&text[start..=i]);
            }
        }
        i += 1;
    }
    None
}

fn unwrap_array(value: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
    if let Some(array) = value.as_array() {
        return Some(array.clone());
    }
    let obj = value.as_object()?;
    for key in [
        "clips",
        "items",
        "hooks",
        "candidates",
        "segments",
        "topics",
    ] {
        if let Some(array) = obj.get(key).and_then(|v| v.as_array()) {
            return Some(array.clone());
        }
    }
    let arrays: Vec<_> = obj.values().filter_map(|v| v.as_array().cloned()).collect();
    if arrays.len() == 1 {
        return Some(arrays.into_iter().next().unwrap());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_text_becomes_a_clip_snapped_to_the_transcript() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 4000,
                text: "Hello".into(),
            },
            TranscriptSegment {
                start_ms: 4500,
                end_ms: 9000,
                text: "World".into(),
            },
        ];
        let text = "Here you go:\n```json\n[{\"title\":\"Hook\",\"start_id\":0,\"end_id\":1,\"category\":\"standalone\"}]\n```";
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Hook");
        assert!(clips[0].end_ms > clips[0].start_ms);
    }

    #[test]
    fn later_clip_list_wins_over_the_example_in_the_prompt() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 4000,
                text: "Hello".into(),
            },
            TranscriptSegment {
                start_ms: 4500,
                end_ms: 9000,
                text: "World".into(),
            },
        ];
        let text = r#"Example: [{"title":"short title","start_id":10,"end_id":18}]
[{"title":"Hook","start_id":"0","end_id":"1","category":"standalone"}]"#;
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Hook");
    }

    #[test]
    fn wrapped_clips_object_is_read() {
        let segments = vec![TranscriptSegment {
            start_ms: 1000,
            end_ms: 5000,
            text: "Line".into(),
        }];
        let text = r#"{"clips":[{"title":"Bit","start_ms":1000,"end_ms":5000}]}"#;
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Bit");
    }
}
