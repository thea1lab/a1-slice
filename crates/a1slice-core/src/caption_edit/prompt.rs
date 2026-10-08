//! The prompt sent to the agent, and the guards on its reply.

use crate::types::{Ms, TranscriptSegment};
use serde_json::Value;

use super::text::{finite_f64, js_floor_nonneg, js_round, js_trim};
use super::types::{CaptionEditRequest, ADDED_TOO_MANY, MOVED_OFF_END, REMOVED_TOO_MANY};
use super::words::{caption_words, even_chunks, format_transcript_text, repair_segment_times};

pub fn clamp_words_per_line(value: f64) -> i64 {
    if !value.is_finite() {
        return 8;
    }
    js_round(value).clamp(2, 24)
}

pub fn format_caption_clock(ms: Ms) -> String {
    let total = js_floor_nonneg(ms as f64 / 1000.0);
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{minutes:02}:{seconds:02}")
}

pub fn caption_line(segment: &TranscriptSegment) -> String {
    format!(
        "{}  {}",
        format_caption_clock(segment.start_ms),
        js_trim(&segment.text)
    )
}

pub fn word_count(segments: &[TranscriptSegment]) -> usize {
    segments
        .iter()
        .map(|segment| caption_words(&segment.text).len())
        .sum()
}

pub fn coerce_segments(value: &Value) -> Option<Vec<TranscriptSegment>> {
    let Value::Array(items) = value else {
        return None;
    };
    let mut segments = Vec::new();
    for item in items {
        if item.is_null() || !item.is_object() {
            return None;
        }
        let Some(start_ms) = item.get("startMs").and_then(finite_f64) else {
            return None;
        };
        let Some(end_ms) = item.get("endMs").and_then(finite_f64) else {
            return None;
        };
        if start_ms < 0.0 {
            return None;
        }
        let text = item
            .get("text")
            .and_then(Value::as_str)
            .map(js_trim)
            .unwrap_or("")
            .to_string();
        if text.is_empty() {
            continue;
        }
        let start = js_round(start_ms);
        let end_rounded = js_round(end_ms);
        let end = if end_rounded > start {
            end_rounded
        } else {
            start + 1
        };
        segments.push(TranscriptSegment {
            start_ms: start,
            end_ms: end,
            text,
        });
    }
    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

pub fn guard_caption_edit(
    before: &[TranscriptSegment],
    after: &[TranscriptSegment],
) -> Option<String> {
    let original = word_count(before);
    let next = word_count(after);
    if original == 0 {
        return None;
    }
    if (next as f64) < (original as f64) * 0.7 {
        return Some(REMOVED_TOO_MANY.to_string());
    }
    if (next as f64) > (original as f64) * 1.3 {
        return Some(ADDED_TOO_MANY.to_string());
    }
    let last = before.last().map(|segment| segment.end_ms).unwrap_or(0);
    if after.iter().any(|segment| segment.start_ms > last + 5000) {
        return Some(MOVED_OFF_END.to_string());
    }
    None
}

pub fn caption_file_text(segments: &[TranscriptSegment]) -> String {
    let body = format_transcript_text(segments);
    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

pub fn lines_longer_than(segments: &[TranscriptSegment], words_per_line: f64) -> usize {
    let limit = clamp_words_per_line(words_per_line) as usize;
    segments
        .iter()
        .filter(|segment| caption_words(&segment.text).len() > limit)
        .count()
}

/// Split lines that are over the word limit, and say what happened.
pub fn split_caption_note(
    segments: &[TranscriptSegment],
    words_per_line: f64,
) -> (Vec<TranscriptSegment>, String) {
    let limit = clamp_words_per_line(words_per_line);
    let too_long = lines_longer_than(segments, words_per_line);
    let next = break_caption_lines(segments, words_per_line);
    let note = if too_long > 0 && next != segments {
        if too_long == 1 {
            format!("Split 1 line longer than {limit} words.")
        } else {
            format!("Split {too_long} lines longer than {limit} words.")
        }
    } else {
        format!("Every line is already {limit} words or shorter.")
    };
    (next, note)
}

pub fn break_caption_lines(
    segments: &[TranscriptSegment],
    words_per_line: f64,
) -> Vec<TranscriptSegment> {
    let limit = clamp_words_per_line(words_per_line) as usize;
    let mut next = Vec::new();
    for segment in segments {
        let parts = caption_words(&segment.text);
        if parts.len() <= limit {
            next.push(segment.clone());
            continue;
        }
        let chunks = even_chunks(&parts, limit);
        let span = segment.end_ms - segment.start_ms;
        let mut at = segment.start_ms;
        for (index, chunk) in chunks.iter().enumerate() {
            let end = if index + 1 == chunks.len() {
                segment.end_ms
            } else {
                at + js_round((span as f64 * chunk.len() as f64) / parts.len() as f64)
            };
            let end_ms = end.max(at + 1);
            next.push(TranscriptSegment {
                start_ms: at,
                end_ms,
                text: chunk.join(" "),
            });
            at = end_ms;
        }
    }
    repair_segment_times(&next)
}

pub fn is_none_answer(output: &str) -> bool {
    let trimmed = js_trim(output).to_lowercase();
    trimmed == "none" || trimmed == "none."
}

pub fn caption_edit_prompt(request: &CaptionEditRequest, file_text: &str) -> String {
    let words = clamp_words_per_line(request.words_per_line);
    let mut jobs = Vec::new();
    if request.fix_typos {
        jobs.push("- Fix typos and words that were clearly misheard.".to_string());
    }
    if request.break_lines {
        jobs.push(format!(
            "- Break any line longer than {words} words into shorter lines. Keep every word, in the same order."
        ));
    }
    let note = js_trim(&request.note);
    if !note.is_empty() {
        jobs.push(format!("- Also do this, and only this: {note}"));
    }
    if jobs.is_empty() {
        jobs.push("- Change nothing unless a line is obviously broken.".to_string());
    }
    let closing = if request.break_lines {
        "- Print only the edited transcript. Do not describe what you are doing."
    } else {
        "- Do not join or split lines. If nothing needs a change, print exactly NONE. Otherwise print only the edited transcript. Do not describe what you are doing."
    };
    let mut lines = vec![
        "Edit this transcript.".to_string(),
        "Each line is one caption. The time stays at the start of the line.".to_string(),
        String::new(),
        "Do only this:".to_string(),
    ];
    lines.extend(jobs);
    lines.push(String::new());
    lines.push("Rules:".to_string());
    lines.push("- Keep the words in the same order.".to_string());
    lines.push("- Do not summarize, translate, or add facts.".to_string());
    lines.push("- When you split a line, leave the time on the first piece only.".to_string());
    lines.push("- Do not add blank lines.".to_string());
    lines.push(closing.to_string());
    lines.push(String::new());
    lines.push(js_trim(file_text).to_string());
    lines.join("\n")
}
