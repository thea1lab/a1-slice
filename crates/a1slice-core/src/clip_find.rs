use crate::types::{Ms, TranscriptSegment};

fn ms_to_timecode(ms: Ms) -> String {
    let total_seconds = ms.div_euclid(1000).max(0);
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

pub fn format_clip_transcript(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .enumerate()
        .map(|(i, seg)| {
            format!(
                "[#{i} {} -> {} | {} -> {}] {}",
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

pub fn clip_find_prompt(segments: &[TranscriptSegment], user_hint: Option<&str>) -> String {
    let hint = user_hint.unwrap_or("").trim();
    let constraint = if hint.is_empty() {
        String::new()
    } else {
        format!("\n\nHard constraint from the user — discard anything that does not match: {hint}")
    };
    format!(
        "You are a video editor. You will receive a numbered transcript. Each line starts with [#N ...]. N is the segment id.\n\n\
Select the best clips from this video.\n\n\
RULES:\n\
- Target duration 20-90 seconds. About 45-60 seconds is ideal.\n\
- Each clip must be a complete thought with a hook and a payoff.\n\
- Use start_id and end_id from the #N ids. Do not invent milliseconds. Do not convert timecodes.\n\
- Skip greetings, filler, and moments that only introduce the class.\n\
- Return 3 to 8 clips. Prefer the strongest moments. Do not cover the whole video.\n\
- Write each title in the language of the transcript.\n\
- Clips must not overlap.\n\n\
CATEGORIES:\n\
- \"related\": about the lecture's main topic\n\
- \"standalone\": understandable with no extra context\n\n\
Return a JSON array and no other text:\n\
[{{\"title\":\"short title\",\"start_id\":10,\"end_id\":18,\"category\":\"standalone\"}}]\n\n\
If nothing is worth clipping, return [].\n\n\
## NUMBERED TRANSCRIPT\n\n\
{}{constraint}",
        format_clip_transcript(segments)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 5000,
                text: "  Hello  ".into(),
            },
            TranscriptSegment {
                start_ms: 5000,
                end_ms: 10000,
                text: "World".into(),
            },
        ]
    }

    #[test]
    fn numbers_each_line_the_way_the_parser_expects() {
        assert_eq!(
            format_clip_transcript(&segments()),
            "[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello\n[#1 00:00:05 -> 00:00:10 | 5000 -> 10000] World"
        );
    }

    #[test]
    fn prompt_asks_for_ids_and_includes_the_hint() {
        let prompt = clip_find_prompt(
            &segments(),
            Some("the part where the elephant confuses the model"),
        );
        assert!(prompt.contains("[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello"));
        assert!(prompt.contains("start_id"));
        assert!(prompt.contains("end_id"));
        assert!(prompt.contains("20-90 seconds"));
        assert!(prompt.contains(
            "Hard constraint from the user — discard anything that does not match: the part where the elephant confuses the model"
        ));
    }

    #[test]
    fn empty_hint_is_left_out() {
        assert!(!clip_find_prompt(&segments(), Some("   ")).contains("Hard constraint"));
    }
}
