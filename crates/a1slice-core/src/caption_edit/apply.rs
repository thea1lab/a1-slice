//! Turn the agent's printed transcript back into timed lines.

use crate::types::TranscriptSegment;
use serde_json::Value;

use super::prompt::{coerce_segments, guard_caption_edit};
use super::text::{
    caption_blocks, first_fence, has_caption_stamp, js_round, js_trim, split_lines, strip_ansi,
    strip_stamp,
};
use super::types::{CaptionEditResult, CaptionLogState, CHANGED_TOO_MUCH, NO_LINES, UNREADABLE};
use super::words::{align_words, caption_words, repair_segment_times, word_timeline};

pub fn caption_file_lines(text: &str) -> Vec<String> {
    split_lines(text)
        .into_iter()
        .map(strip_stamp)
        .filter(|line| !line.is_empty())
        .collect()
}

pub fn apply_edited_caption_text(
    before: &[TranscriptSegment],
    edited_text: &str,
) -> CaptionEditResult {
    let lines = caption_file_lines(edited_text);
    if lines.is_empty() {
        return CaptionEditResult::Error(NO_LINES.to_string());
    }
    let timeline = word_timeline(before);
    if timeline.is_empty() {
        return CaptionEditResult::Error(UNREADABLE.to_string());
    }
    let edited_words: Vec<String> = lines.iter().flat_map(|line| caption_words(line)).collect();
    let original_words: Vec<String> = timeline.iter().map(|item| item.word.clone()).collect();
    let Some(aligned) = align_words(&original_words, &edited_words) else {
        return CaptionEditResult::Error(CHANGED_TOO_MUCH.to_string());
    };
    if aligned.len() != edited_words.len() {
        return CaptionEditResult::Error(CHANGED_TOO_MUCH.to_string());
    }

    let mut built = Vec::new();
    let mut owners = Vec::new();
    let mut cursor = 0;
    for line in &lines {
        let count = caption_words(line).len();
        if count == 0 || cursor + count > aligned.len() {
            return CaptionEditResult::Error(UNREADABLE.to_string());
        }
        let indices = &aligned[cursor..cursor + count];
        cursor += count;
        let mut owner = timeline[indices[0]].segment_index as i64;
        if indices
            .iter()
            .any(|&index| timeline[index].segment_index as i64 != owner)
        {
            owner = -1;
        }
        owners.push(owner);
        let first = &timeline[indices[0]];
        let last = &timeline[indices[indices.len() - 1]];
        built.push(TranscriptSegment {
            start_ms: first.start_ms,
            end_ms: last.end_ms.max(first.start_ms + 1),
            text: line.clone(),
        });
    }

    let mut line_index = 0;
    while line_index < built.len() {
        let owner = owners[line_index];
        if owner < 0 {
            line_index += 1;
            continue;
        }
        let mut end = line_index + 1;
        while end < built.len() && owners[end] == owner {
            end += 1;
        }
        let segment = &before[owner as usize];
        let counts: Vec<usize> = built[line_index..end]
            .iter()
            .map(|item| caption_words(&item.text).len())
            .collect();
        let total: usize = counts.iter().sum();
        if total == 0 {
            line_index = end;
            continue;
        }
        let span = segment.end_ms - segment.start_ms;
        let mut at = segment.start_ms;
        for (index, item) in built[line_index..end].iter_mut().enumerate() {
            let next = if index + 1 == counts.len() {
                segment.end_ms
            } else {
                at + js_round((span as f64 * counts[index] as f64) / total as f64)
            };
            item.start_ms = at;
            item.end_ms = next.max(at + 1);
            at = item.end_ms;
        }
        line_index = end;
    }

    let built = repair_segment_times(&built);
    if let Some(message) = guard_caption_edit(before, &built) {
        return CaptionEditResult::Error(message);
    }
    CaptionEditResult::Segments(built)
}

pub fn extract_printed_caption(output: &str) -> Option<String> {
    let blocks = if let Some(fenced) = first_fence(output, &["text", "txt"]) {
        let inner = caption_blocks(fenced);
        if !inner.is_empty() {
            inner
        } else {
            caption_blocks(output)
        }
    } else {
        caption_blocks(output)
    };
    blocks.last().cloned()
}

pub fn caption_log_line(line: &str, state: &mut CaptionLogState) -> Option<String> {
    let trimmed = js_trim(&strip_ansi(line)).to_string();
    if trimmed.is_empty() {
        return None;
    }
    if has_caption_stamp(&trimmed) || state.hiding_file {
        let started = state.hiding_file;
        state.hiding_file = true;
        if started {
            None
        } else {
            Some("Writing the edited transcript.".to_string())
        }
    } else {
        Some(trimmed)
    }
}

pub fn parse_agent_caption_edit(output: &str) -> Result<Value, String> {
    let mut candidates = Vec::new();
    let start = output.find("CAPTIONS_JSON_START");
    let end = output.find("CAPTIONS_JSON_END");
    if let (Some(start), Some(end)) = (start, end) {
        if end > start {
            let from = start + "CAPTIONS_JSON_START".len();
            let slice = if end >= from { &output[from..end] } else { "" };
            candidates.push(js_trim(slice).to_string());
        }
    }
    if let Some(fenced) = first_fence(output, &["json"]) {
        candidates.push(js_trim(fenced).to_string());
    }
    if let (Some(first), Some(last)) = (output.find('['), output.rfind(']')) {
        if last > first {
            candidates.push(output[first..=last].to_string());
        }
    }
    for candidate in candidates {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            return Ok(value);
        }
    }
    Err("no json".to_string())
}

pub fn accept_agent_edit(before: &[TranscriptSegment], output: &str) -> CaptionEditResult {
    let parsed = match parse_agent_caption_edit(output) {
        Ok(value) => value,
        Err(_) => return CaptionEditResult::Error(NO_LINES.to_string()),
    };
    let Some(segments) = coerce_segments(&parsed) else {
        return CaptionEditResult::Error(UNREADABLE.to_string());
    };
    if let Some(message) = guard_caption_edit(before, &segments) {
        return CaptionEditResult::Error(message);
    }
    CaptionEditResult::Segments(segments)
}
