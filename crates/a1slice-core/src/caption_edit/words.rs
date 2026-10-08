//! Align edited words back onto the original timestamps.

use crate::types::{Ms, TranscriptSegment};

use super::text::{js_floor_nonneg, js_round, js_trim, split_js_ws};

pub(super) fn format_transcript_text(segments: &[TranscriptSegment]) -> String {
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

pub(super) fn repair_segment_times(segments: &[TranscriptSegment]) -> Vec<TranscriptSegment> {
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

pub(super) fn even_chunks(words: &[String], limit: usize) -> Vec<Vec<String>> {
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

pub(super) fn caption_words(text: &str) -> Vec<String> {
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

pub(super) struct TimedWord {
    pub(super) word: String,
    pub(super) start_ms: Ms,
    pub(super) end_ms: Ms,
    pub(super) segment_index: usize,
}

pub(super) fn word_timeline(segments: &[TranscriptSegment]) -> Vec<TimedWord> {
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

pub(super) fn align_words(original: &[String], edited: &[String]) -> Option<Vec<usize>> {
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
