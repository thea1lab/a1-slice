//! SRT text and the preview-cache key.

use crate::preview::preview_cache_key;
use crate::types::{Ms, TranscriptSegment};
use std::fs;
use std::path::Path;

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
