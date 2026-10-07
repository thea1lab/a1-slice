//! Edit caption lines: prompts, word alignment, diffs, and the agent log.

use crate::types::{Ms, TranscriptSegment};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionEditRequest {
    pub fix_typos: bool,
    pub break_lines: bool,
    pub words_per_line: f64,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Same,
    Add,
    Remove,
    Gap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptionEditResult {
    Segments(Vec<TranscriptSegment>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionLogState {
    pub hiding_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLogState {
    pub hide_json: bool,
    pub final_text: Option<String>,
    pub pieces: Vec<String>,
}

const NO_LINES: &str =
    "The agent did not return the edited lines. The lines were left as they are.";
const UNREADABLE: &str =
    "The agent returned lines that could not be read. The lines were left as they are.";
const CHANGED_TOO_MUCH: &str =
    "The agent changed too much of the file. The lines were left as they are.";
const REMOVED_TOO_MANY: &str = "The agent removed too many words. The lines were left as they are.";
const ADDED_TOO_MANY: &str = "The agent added too many words. The lines were left as they are.";
const MOVED_OFF_END: &str =
    "The agent moved a line off the end of the video. The lines were left as they are.";

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

pub fn diff_caption_lines<A: AsRef<str>, B: AsRef<str>>(before: &[A], after: &[B]) -> Vec<DiffRow> {
    let n = before.len();
    let m = after.len();
    let mut scores = vec![vec![0i64; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            scores[i][j] = if before[i].as_ref() == after[j].as_ref() {
                scores[i + 1][j + 1] + 1
            } else {
                scores[i + 1][j].max(scores[i][j + 1])
            };
        }
    }
    let mut rows = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < n && j < m {
        if before[i].as_ref() == after[j].as_ref() {
            rows.push(DiffRow {
                kind: DiffKind::Same,
                text: before[i].as_ref().to_string(),
            });
            i += 1;
            j += 1;
        } else if scores[i + 1][j] >= scores[i][j + 1] {
            rows.push(DiffRow {
                kind: DiffKind::Remove,
                text: before[i].as_ref().to_string(),
            });
            i += 1;
        } else {
            rows.push(DiffRow {
                kind: DiffKind::Add,
                text: after[j].as_ref().to_string(),
            });
            j += 1;
        }
    }
    while i < n {
        rows.push(DiffRow {
            kind: DiffKind::Remove,
            text: before[i].as_ref().to_string(),
        });
        i += 1;
    }
    while j < m {
        rows.push(DiffRow {
            kind: DiffKind::Add,
            text: after[j].as_ref().to_string(),
        });
        j += 1;
    }
    rows
}

pub fn compact_diff(rows: &[DiffRow], context: i64) -> Vec<DiffRow> {
    let mut keep = vec![false; rows.len()];
    for (index, row) in rows.iter().enumerate() {
        if row.kind == DiffKind::Same {
            continue;
        }
        let last = rows.len() as i64 - 1;
        let from = (index as i64 - context).max(0);
        let to = (index as i64 + context).min(last);
        if to < from {
            continue;
        }
        for cursor in from..=to {
            keep[cursor as usize] = true;
        }
    }
    let mut compact = Vec::new();
    let mut skipped = false;
    for (index, row) in rows.iter().enumerate() {
        if !keep[index] {
            skipped = true;
            continue;
        }
        if skipped {
            compact.push(DiffRow {
                kind: DiffKind::Gap,
                text: "…".to_string(),
            });
            skipped = false;
        }
        compact.push(row.clone());
    }
    compact
}

pub fn diff_summary(rows: &[DiffRow]) -> String {
    let removed = rows
        .iter()
        .filter(|row| row.kind == DiffKind::Remove)
        .count();
    let added = rows.iter().filter(|row| row.kind == DiffKind::Add).count();
    if removed == 0 && added == 0 {
        return "The agent left the words as they were.".to_string();
    }
    let mut parts = Vec::new();
    if removed > 0 {
        parts.push(if removed == 1 {
            "Removed 1 line.".to_string()
        } else {
            format!("Removed {removed} lines.")
        });
    }
    if added > 0 {
        parts.push(if added == 1 {
            "Added 1 line.".to_string()
        } else {
            format!("Added {added} lines.")
        });
    }
    parts.join(" ")
}

pub fn empty_agent_log_state() -> AgentLogState {
    AgentLogState {
        hide_json: false,
        final_text: None,
        pieces: Vec::new(),
    }
}

pub fn visible_log_line(line: &str, hide_json: &mut bool) -> Option<String> {
    let trimmed = js_trim(&strip_ansi(line)).to_string();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("CAPTIONS_JSON_START") {
        *hide_json = true;
        return Some("Writing the edited captions.".to_string());
    }
    if trimmed.contains("CAPTIONS_JSON_END") {
        *hide_json = false;
        return None;
    }
    if *hide_json || trimmed.starts_with("```") {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("NOTE:") {
        return Some(js_trim(rest).to_string());
    }
    Some(trimmed)
}

pub fn interpret_agent_line(
    line: &str,
    state: &mut AgentLogState,
    stream_json: bool,
) -> Vec<String> {
    if !stream_json {
        state.pieces.push(line.to_string());
        return visible_log_line(line, &mut state.hide_json)
            .into_iter()
            .collect();
    }
    let event: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => {
            return visible_log_line(line, &mut state.hide_json)
                .into_iter()
                .collect()
        }
    };
    if event.get("type").and_then(Value::as_str) == Some("result") {
        if let Some(result) = event.get("result").and_then(Value::as_str) {
            state.final_text = Some(result.to_string());
            if !state.pieces.is_empty() {
                return Vec::new();
            }
            let mut logs = Vec::new();
            for part in split_lines(result) {
                if let Some(shown) = visible_log_line(part, &mut state.hide_json) {
                    logs.push(shown);
                }
            }
            return logs;
        }
    }
    if event.get("type").and_then(Value::as_str) != Some("assistant") {
        return Vec::new();
    }
    let Some(message) = event.get("message") else {
        return Vec::new();
    };
    if message.is_null() || !message.is_object() {
        return Vec::new();
    }
    let Some(content) = message.get("content").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut logs = Vec::new();
    for block in content {
        let Some(row) = block.as_object() else {
            continue;
        };
        if row.get("type").and_then(Value::as_str) == Some("tool_use") {
            let name = row
                .get("name")
                .map(js_string)
                .unwrap_or_else(|| "undefined".to_string());
            logs.push(if name == "Read" {
                "Reading the captions.".to_string()
            } else {
                format!("Using {name}.")
            });
        }
        if row.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(text) = row.get("text").and_then(Value::as_str) {
                state.pieces.push(text.to_string());
                for part in split_lines(text) {
                    if let Some(shown) = visible_log_line(part, &mut state.hide_json) {
                        logs.push(shown);
                    }
                }
            }
        }
    }
    logs
}

pub fn agent_output_text(state: &AgentLogState) -> String {
    if let Some(text) = &state.final_text {
        return text.clone();
    }
    state.pieces.join("\n")
}

fn format_transcript_text(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|segment| {
            let total = js_floor_nonneg(segment.start_ms as f64 / 1000.0);
            let hours = total / 3600;
            let minutes = (total % 3600) / 60;
            let seconds = total % 60;
            let stamp = if hours > 0 {
                format!("{hours}:{minutes:02}:{seconds:02}")
            } else {
                format!("{minutes}:{seconds:02}")
            };
            format!("{stamp}  {}", js_trim(&segment.text))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn repair_segment_times(segments: &[TranscriptSegment]) -> Vec<TranscriptSegment> {
    let mut next = segments.to_vec();
    for index in 0..next.len() {
        if index > 0 {
            let start = next[index].start_ms;
            let prev_start = next[index - 1].start_ms;
            let prev_end = next[index - 1].end_ms;
            if start < prev_end {
                if start > prev_start {
                    next[index - 1].end_ms = start;
                } else {
                    next[index].start_ms = prev_end;
                }
            }
        }
        if next[index].end_ms <= next[index].start_ms {
            next[index].end_ms = next[index].start_ms + 1;
        }
    }
    next
}

fn even_chunks(words: &[String], limit: usize) -> Vec<Vec<String>> {
    let chunks = words.len().div_ceil(limit);
    let base = words.len() / chunks;
    let extra = words.len() % chunks;
    let mut result = Vec::new();
    let mut cursor = 0;
    for index in 0..chunks {
        let size = base + usize::from(index < extra);
        result.push(words[cursor..cursor + size].to_vec());
        cursor += size;
    }
    result
}

fn caption_words(text: &str) -> Vec<String> {
    let trimmed = js_trim(text);
    if trimmed.is_empty() {
        return Vec::new();
    }
    split_js_ws(trimmed)
        .into_iter()
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn same_word(left: &str, right: &str) -> bool {
    let a = clean_word(left);
    let b = clean_word(right);
    !a.is_empty() && a == b
}

fn clean_word(word: &str) -> String {
    word.to_lowercase()
        .chars()
        .filter(|c| *c == '\'' || c.is_alphanumeric())
        .collect()
}

struct TimedWord {
    word: String,
    start_ms: Ms,
    end_ms: Ms,
    segment_index: usize,
}

fn word_timeline(segments: &[TranscriptSegment]) -> Vec<TimedWord> {
    let mut timeline = Vec::new();
    for (segment_index, segment) in segments.iter().enumerate() {
        let parts = caption_words(&segment.text);
        if parts.is_empty() {
            continue;
        }
        let span = segment.end_ms - segment.start_ms;
        let len = parts.len() as f64;
        for (index, word) in parts.into_iter().enumerate() {
            let start_ms = js_round(segment.start_ms as f64 + (span as f64 * index as f64) / len);
            let end_ms =
                js_round(segment.start_ms as f64 + (span as f64 * (index as f64 + 1.0)) / len);
            timeline.push(TimedWord {
                word,
                start_ms,
                end_ms: end_ms.max(start_ms + 1),
                segment_index,
            });
        }
    }
    timeline
}

fn align_words(original: &[String], edited: &[String]) -> Option<Vec<usize>> {
    let mut map = Vec::new();
    let mut i = 0;
    let mut j = 0;
    let mut chaos: i64 = 0;
    while j < edited.len() {
        if i >= original.len() {
            map.push(original.len().saturating_sub(1));
            chaos += 1;
            j += 1;
            continue;
        }
        if same_word(&original[i], &edited[j]) {
            map.push(i);
            i += 1;
            j += 1;
            continue;
        }
        let mut ahead: i64 = -1;
        for step in 1..=3 {
            if i + step < original.len() && same_word(&original[i + step], &edited[j]) {
                ahead = (i + step) as i64;
                break;
            }
        }
        if ahead >= 0 {
            chaos += ahead - i as i64;
            i = ahead as usize;
            map.push(i);
            i += 1;
            j += 1;
            continue;
        }
        let mut inserted: i64 = -1;
        for step in 1..=3 {
            if j + step < edited.len() && same_word(&original[i], &edited[j + step]) {
                inserted = step as i64;
                break;
            }
        }
        if inserted >= 0 {
            for _ in 0..inserted {
                map.push(i);
                chaos += 1;
                j += 1;
            }
            continue;
        }
        map.push(i);
        i += 1;
        j += 1;
    }
    chaos += original.len() as i64 - i as i64;
    if !original.is_empty() && (chaos as f64) > (original.len() as f64) * 0.35 {
        return None;
    }
    Some(map)
}

fn strip_stamp(line: &str) -> String {
    match match_time_prefix(line) {
        Some(index) => js_trim(&line[index..]).to_string(),
        None => js_trim(line).to_string(),
    }
}

fn has_caption_stamp(line: &str) -> bool {
    match_time_prefix(line).is_some_and(|index| index < line.len())
}

fn match_time_prefix(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut digits = 0;
    while digits < bytes.len() && bytes[digits].is_ascii_digit() {
        digits += 1;
    }
    if digits > 0 && digits < bytes.len() && bytes[digits] == b':' {
        if let Some(index) = match_min_sec(line, digits + 1) {
            return Some(index);
        }
    }
    match_min_sec(line, 0)
}

fn match_min_sec(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if start >= bytes.len() || !bytes[start].is_ascii_digit() {
        return None;
    }
    let mut consumed = start + 1;
    if consumed < bytes.len() && bytes[consumed].is_ascii_digit() {
        consumed += 1;
    }
    if let Some(index) = after_minutes(line, consumed) {
        return Some(index);
    }
    if consumed == start + 2 {
        after_minutes(line, start + 1)
    } else {
        None
    }
}

fn after_minutes(line: &str, colon_at: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if colon_at >= bytes.len() || bytes[colon_at] != b':' {
        return None;
    }
    let seconds = colon_at + 1;
    if seconds + 1 >= bytes.len()
        || !bytes[seconds].is_ascii_digit()
        || !bytes[seconds + 1].is_ascii_digit()
    {
        return None;
    }
    let after = seconds + 2;
    let whitespace = leading_ws_len(&line[after..]);
    if whitespace == 0 {
        return None;
    }
    Some(after + whitespace)
}

fn strip_ansi(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b';') {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'm' {
                i = j + 1;
                continue;
            }
        }
        let ch = text[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn caption_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in split_lines(&strip_ansi(text)) {
        if js_trim(line).is_empty() {
            if let Some(cur) = current.take() {
                if !cur.is_empty() {
                    let joined = cur.join("\n");
                    blocks.push(js_trim(&joined).to_string());
                }
            }
            continue;
        }
        if let Some(cur) = current.as_mut() {
            cur.push(line.to_string());
            continue;
        }
        if has_caption_stamp(js_trim(line)) {
            current = Some(vec![line.to_string()]);
        }
    }
    if let Some(cur) = current.take() {
        if !cur.is_empty() {
            let joined = cur.join("\n");
            blocks.push(js_trim(&joined).to_string());
        }
    }
    blocks
        .into_iter()
        .filter(|block| !caption_file_lines(block).is_empty())
        .collect()
}

fn first_fence<'a>(output: &'a str, langs: &[&str]) -> Option<&'a str> {
    let mut index = 0;
    while index < output.len() {
        if output[index..].starts_with("```") {
            if let Some(content) = fence_at(output, index + 3, langs) {
                return Some(content);
            }
        }
        index += output[index..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
    }
    None
}

fn fence_at<'a>(output: &'a str, after_ticks: usize, langs: &[&str]) -> Option<&'a str> {
    let rest = &output[after_ticks..];
    let mut options: Vec<&str> = langs.to_vec();
    options.push("");
    for lang in options {
        if !rest.starts_with(lang) {
            continue;
        }
        let mut content_at = after_ticks + lang.len();
        content_at += leading_ws_len(&output[content_at..]);
        if let Some(close) = output[content_at..].find("```") {
            return Some(&output[content_at..content_at + close]);
        }
    }
    None
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                int.to_string()
            } else if let Some(uint) = number.as_u64() {
                uint.to_string()
            } else if let Some(float) = number.as_f64() {
                format!("{float}")
            } else {
                "undefined".to_string()
            }
        }
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn finite_f64(value: &Value) -> Option<f64> {
    let number = value.as_f64()?;
    number.is_finite().then_some(number)
}

fn js_round(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    let floor = n.floor();
    let rounded = if n - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded >= i64::MAX as f64 {
        i64::MAX
    } else if rounded <= i64::MIN as f64 {
        i64::MIN
    } else {
        rounded as i64
    }
}

fn js_floor_nonneg(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    let floor = n.floor();
    if floor <= 0.0 {
        0
    } else if floor >= i64::MAX as f64 {
        i64::MAX
    } else {
        floor as i64
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    let Some(start) = s.find(|c: char| !is_js_ws(c)) else {
        return "";
    };
    let end = s
        .rfind(|c: char| !is_js_ws(c))
        .map(|index| index + s[index..].chars().next().map(|c| c.len_utf8()).unwrap_or(0))
        .unwrap_or(s.len());
    &s[start..end]
}

fn leading_ws_len(s: &str) -> usize {
    let mut size = 0;
    for c in s.chars() {
        if !is_js_ws(c) {
            break;
        }
        size += c.len_utf8();
    }
    size
}

fn split_js_ws(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (index, c) in s.char_indices() {
        if is_js_ws(c) {
            if let Some(from) = start.take() {
                out.push(&s[from..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        out.push(&s[from..]);
    }
    out
}

fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let mut end = i;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
            lines.push(&text[start..end]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    lines.push(&text[start..]);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seg(start_ms: Ms, end_ms: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    fn lines() -> Vec<TranscriptSegment> {
        vec![
            seg(0, 2000, "Hello teh world"),
            seg(2000, 6000, "This line is much too long for one caption"),
        ]
    }

    #[test]
    fn asks_only_for_typos_shorter_lines_and_the_note() {
        let prompt = caption_edit_prompt(
            &CaptionEditRequest {
                fix_typos: true,
                break_lines: true,
                words_per_line: 8.0,
                note: "The name is Anna".to_string(),
            },
            &caption_file_text(&lines()),
        );
        assert!(prompt.contains("Edit this transcript."));
        assert!(prompt.contains("Fix typos"));
        assert!(prompt.contains("longer than 8 words"));
        assert!(prompt.contains("The name is Anna"));
        assert!(prompt.contains("Print only the edited transcript."));
        assert!(prompt.contains("0:00  Hello teh world"));
        assert!(!prompt.contains("captions.txt"));
        assert!(!prompt.contains("CAPTIONS_JSON"));
    }

    #[test]
    fn asks_for_none_when_it_should_not_break_lines() {
        let prompt = caption_edit_prompt(
            &CaptionEditRequest {
                fix_typos: true,
                break_lines: false,
                words_per_line: 8.0,
                note: String::new(),
            },
            "0:00  Hello teh world",
        );
        assert!(prompt.contains("print exactly NONE"));
        assert!(prompt.contains("Do not join or split lines."));
        assert!(is_none_answer("NONE"));
        assert!(!is_none_answer("I'll split the line."));
    }

    #[test]
    fn splits_a_long_line_into_even_pieces_and_keeps_every_word() {
        let segment = lines().into_iter().nth(1).unwrap();
        assert_eq!(
            break_caption_lines(&[segment], 8.0),
            vec![
                seg(2000, 4222, "This line is much too"),
                seg(4222, 6000, "long for one caption"),
            ]
        );
    }

    #[test]
    fn leaves_a_line_that_is_already_short_enough() {
        let segment = seg(0, 1000, "one two three four five six seven eight");
        assert_eq!(break_caption_lines(&[segment.clone()], 8.0), vec![segment]);
    }

    #[test]
    fn keeps_a_line_that_starts_while_the_previous_line_is_still_going() {
        let before = vec![
            seg(
                171_000,
                176_500,
                "Mas só você seleciona clique aqui e selecione as imagens que vocês querem",
            ),
            seg(173_960, 175_840, "e selecionem as imagens"),
        ];
        let split = break_caption_lines(&before, 8.0);
        assert!(split
            .iter()
            .all(|segment| segment.end_ms > segment.start_ms));
        let CaptionEditResult::Segments(parsed) =
            apply_edited_caption_text(&before, &caption_file_text(&split))
        else {
            panic!("expected segments");
        };
        assert!(parsed
            .iter()
            .all(|segment| segment.end_ms > segment.start_ms));
        let value = serde_json::to_value(&parsed).unwrap();
        let coerced = coerce_segments(&value).unwrap();
        let joined = coerced
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("e selecionem as imagens"));
    }

    #[test]
    fn breaks_a_very_long_line_into_pieces_of_at_most_eight_words() {
        let words: Vec<String> = (1..=31).map(|index| format!("w{index}")).collect();
        let result = break_caption_lines(&[seg(0, 31_000, &words.join(" "))], 8.0);
        assert_eq!(
            result
                .iter()
                .map(|segment| segment.text.split(' ').count())
                .collect::<Vec<_>>(),
            vec![8, 8, 8, 7]
        );
        assert_eq!(
            result
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            words.join(" ")
        );
        assert_eq!(result[0].start_ms, 0);
        assert_eq!(result.last().unwrap().end_ms, 31_000);
    }

    #[test]
    fn keeps_the_times_when_the_file_only_fixes_a_typo() {
        let edited = caption_file_text(&[
            seg(0, 2000, "Hello the world"),
            seg(2000, 6000, "This line is much too long for one caption"),
        ]);
        assert_eq!(
            apply_edited_caption_text(&lines(), &edited),
            CaptionEditResult::Segments(vec![
                seg(0, 2000, "Hello the world"),
                seg(2000, 6000, "This line is much too long for one caption"),
            ])
        );
    }

    #[test]
    fn splits_the_time_when_a_long_line_becomes_two_lines() {
        let edited = [
            "0:00  Hello the world",
            "0:02  This line is much",
            "too long for one caption",
        ]
        .join("\n");
        assert_eq!(
            apply_edited_caption_text(&lines(), &edited),
            CaptionEditResult::Segments(vec![
                seg(0, 2000, "Hello the world"),
                seg(2000, 3778, "This line is much"),
                seg(3778, 6000, "too long for one caption"),
            ])
        );
    }

    #[test]
    fn refuses_a_rewrite_that_drops_most_of_the_words() {
        assert_eq!(
            apply_edited_caption_text(&lines(), "0:00  Hello\n"),
            CaptionEditResult::Error(CHANGED_TOO_MUCH.to_string())
        );
    }

    #[test]
    fn keeps_the_original_times_when_the_file_is_unchanged() {
        let source = lines();
        assert_eq!(
            apply_edited_caption_text(&source, &caption_file_text(&source)),
            CaptionEditResult::Segments(source)
        );
    }

    #[test]
    fn keeps_a_split_line_that_has_no_time_of_its_own() {
        let printed = extract_printed_caption(
            &[
                "I will fix the typo.",
                "0:00  Hello the world",
                "0:02  This line is much too long",
                "for one caption",
                "",
                "Done.",
            ]
            .join("\n"),
        );
        let expected = [
            "0:00  Hello the world",
            "0:02  This line is much too long",
            "for one caption",
        ]
        .join("\n");
        assert_eq!(printed.as_deref(), Some(expected.as_str()));
        assert_eq!(
            apply_edited_caption_text(&lines(), printed.as_deref().unwrap_or("")),
            CaptionEditResult::Segments(vec![
                seg(0, 2000, "Hello the world"),
                seg(2000, 4667, "This line is much too long"),
                seg(4667, 6000, "for one caption"),
            ])
        );
    }

    #[test]
    fn uses_the_last_transcript_when_an_earlier_copy_is_still_in_the_answer() {
        let printed = extract_printed_caption(
            &[
                "0:00  Hello teh world",
                "0:02  This line is much too long for one caption",
                "",
                "0:00  Hello the world",
                "0:02  This line is much too long",
                "for one caption",
            ]
            .join("\n"),
        );
        assert_eq!(
            printed.as_deref(),
            Some(
                [
                    "0:00  Hello the world",
                    "0:02  This line is much too long",
                    "for one caption"
                ]
                .join("\n")
                .as_str()
            )
        );
    }

    #[test]
    fn shows_the_note_and_hides_the_printed_transcript() {
        let mut state = CaptionLogState { hiding_file: false };
        assert_eq!(
            caption_log_line("I will fix the typo.", &mut state).as_deref(),
            Some("I will fix the typo.")
        );
        assert_eq!(
            caption_log_line("0:00  Hello the world", &mut state).as_deref(),
            Some("Writing the edited transcript.")
        );
        assert_eq!(caption_log_line("for one caption", &mut state), None);
    }

    #[test]
    fn reads_the_json_block_and_keeps_a_small_correction() {
        let output = [
            "NOTE: At 00:00, fixed \"teh\" to \"the\".",
            "CAPTIONS_JSON_START",
            &json!([
                { "startMs": 0, "endMs": 2000, "text": "Hello the world" },
                { "startMs": 2000, "endMs": 4000, "text": "This line is much" },
                { "startMs": 4000, "endMs": 6000, "text": "too long for one caption" }
            ])
            .to_string(),
            "CAPTIONS_JSON_END",
        ]
        .join("\n");
        assert_eq!(
            accept_agent_edit(&lines(), &output),
            CaptionEditResult::Segments(vec![
                seg(0, 2000, "Hello the world"),
                seg(2000, 4000, "This line is much"),
                seg(4000, 6000, "too long for one caption"),
            ])
        );
    }

    #[test]
    fn accept_refuses_a_rewrite_that_drops_most_of_the_words() {
        let output = format!(
            "CAPTIONS_JSON_START\n{}\nCAPTIONS_JSON_END",
            json!([{ "startMs": 0, "endMs": 2000, "text": "Hello" }])
        );
        assert_eq!(
            accept_agent_edit(&lines(), &output),
            CaptionEditResult::Error(REMOVED_TOO_MANY.to_string())
        );
    }

    #[test]
    fn hides_the_json_from_the_log_and_keeps_the_notes() {
        let mut hide_json = false;
        assert_eq!(
            visible_log_line("NOTE: At 00:00, fixed \"teh\" to \"the\".", &mut hide_json)
                .as_deref(),
            Some("At 00:00, fixed \"teh\" to \"the\".")
        );
        assert_eq!(
            visible_log_line("CAPTIONS_JSON_START", &mut hide_json).as_deref(),
            Some("Writing the edited captions.")
        );
        assert_eq!(
            visible_log_line("[{\"text\":\"hidden\"}]", &mut hide_json),
            None
        );
        assert_eq!(visible_log_line("CAPTIONS_JSON_END", &mut hide_json), None);
    }

    #[test]
    fn shows_a_removed_line_and_the_line_that_replaced_it() {
        let rows = diff_caption_lines(&["00:00  Hello teh"], &["00:00  Hello the"]);
        assert_eq!(
            rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
            vec![DiffKind::Remove, DiffKind::Add]
        );
        assert_eq!(diff_summary(&rows), "Removed 1 line. Added 1 line.");
        let compact = compact_diff(
            &diff_caption_lines(
                &["00:00  Same", "00:02  Old", "00:04  After"],
                &["00:00  Same", "00:02  New", "00:04  After"],
            ),
            0,
        );
        assert_eq!(
            compact.iter().map(|row| row.kind).collect::<Vec<_>>(),
            vec![DiffKind::Gap, DiffKind::Remove, DiffKind::Add]
        );
    }

    #[test]
    fn turns_a_claude_tool_call_into_a_plain_sentence_and_keeps_the_result_text() {
        let mut state = empty_agent_log_state();
        let reading = interpret_agent_line(
            &json!({
                "type": "assistant",
                "message": { "content": [{ "type": "tool_use", "name": "Read" }] }
            })
            .to_string(),
            &mut state,
            true,
        );
        assert_eq!(reading, vec!["Reading the captions.".to_string()]);
        let finished = interpret_agent_line(
            &json!({
                "type": "result",
                "result": "NOTE: At 00:00, fixed a typo.\nCAPTIONS_JSON_START\n[]\nCAPTIONS_JSON_END"
            })
            .to_string(),
            &mut state,
            true,
        );
        assert_eq!(
            finished,
            vec![
                "At 00:00, fixed a typo.".to_string(),
                "Writing the edited captions.".to_string()
            ]
        );
        assert!(agent_output_text(&state).contains("CAPTIONS_JSON_START"));
    }

    #[test]
    fn does_not_repeat_notes_that_were_already_shown() {
        let mut state = empty_agent_log_state();
        interpret_agent_line(
            &json!({
                "type": "assistant",
                "message": { "content": [{ "type": "text", "text": "NOTE: At 00:00, fixed a typo." }] }
            })
            .to_string(),
            &mut state,
            true,
        );
        let again = interpret_agent_line(
            &json!({
                "type": "result",
                "result": "NOTE: At 00:00, fixed a typo.\nCAPTIONS_JSON_START\n[]\nCAPTIONS_JSON_END"
            })
            .to_string(),
            &mut state,
            true,
        );
        assert_eq!(again, Vec::<String>::new());
        assert!(agent_output_text(&state).contains("CAPTIONS_JSON_START"));
    }
}
