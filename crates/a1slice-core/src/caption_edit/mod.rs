//! Edit caption lines: prompts, word alignment, diffs, and the agent log.

mod apply;
mod diff;
mod log;
mod prompt;
mod text;
mod types;
mod words;

pub use apply::{
    accept_agent_edit, apply_edited_caption_text, caption_file_lines, caption_log_line,
    extract_printed_caption, parse_agent_caption_edit,
};
pub use diff::{compact_diff, diff_caption_lines, diff_summary};
pub use log::{agent_output_text, empty_agent_log_state, interpret_agent_line, visible_log_line};
pub use prompt::{
    break_caption_lines, caption_edit_prompt, caption_file_text, caption_line,
    clamp_words_per_line, coerce_segments, format_caption_clock, guard_caption_edit,
    is_none_answer, word_count,
};
pub use types::{
    AgentLogState, CaptionEditRequest, CaptionEditResult, CaptionLogState, DiffKind, DiffRow,
};

#[cfg(test)]
mod tests {
    use super::types::{CHANGED_TOO_MUCH, REMOVED_TOO_MANY};
    use super::*;
    use crate::types::{Ms, TranscriptSegment};
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
