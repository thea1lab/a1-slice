//! Installed caption agents, and parsing the clip JSON they print.

mod launch;
mod parse;

pub use launch::{
    agent_launch, command_on_path, list_installed_agents, AgentLaunch, CaptionAgent,
    CaptionAgentId, CaptionAgentInfo, CAPTION_AGENTS,
};
pub use parse::{
    clips_from_agent_text, extract_json_array, format_transcript_for_llm, ms_to_timecode,
    parse_llm_response, NoJsonArray,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::types::{ClipCategory, ClipSegment, Ms, TranscriptSegment};
    use serde_json::json;
    fn seg(start_ms: Ms, end_ms: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    fn clip(title: &str, start_ms: Ms, end_ms: Ms, category: Option<ClipCategory>) -> ClipSegment {
        ClipSegment {
            title: title.into(),
            start_ms,
            end_ms,
            category,
            topic: None,
            crop: None,
            approved: None,
        }
    }

    fn arg_after<'a>(args: &'a [String], flag: &str) -> &'a str {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        &args[index + 1]
    }

    #[test]
    fn grok_prints_in_one_turn_at_low_effort() {
        let launch = agent_launch(CaptionAgentId::Grok, "Edit this transcript.", "/tmp/job");
        assert_eq!(launch.command, "grok");
        assert!(launch.args.iter().any(|arg| arg == "--no-plan"));
        assert!(launch.args.iter().any(|arg| arg == "grok-4.7"));
        assert_eq!(arg_after(&launch.args, "--reasoning-effort"), "low");
        assert!(!launch.args.iter().any(|arg| arg == "plan"));
        assert!(!launch.args.iter().any(|arg| arg == "acceptEdits"));
        assert!(!launch.args.iter().any(|arg| arg == "read,edit"));
        assert_eq!(arg_after(&launch.args, "-p"), "Edit this transcript.");
        assert_eq!(launch.stdin, None);
        assert!(!launch.stream_json);
    }

    #[test]
    fn grok_uses_a_prompt_file_when_the_transcript_is_very_long() {
        let prompt = "x".repeat(100_000);
        let launch = agent_launch(CaptionAgentId::Grok, &prompt, "/tmp/job");
        let instructions = Path::new("/tmp/job")
            .join("instructions.md")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            &launch.args[..2],
            &["--prompt-file".to_string(), instructions]
        );
        assert!(launch.args.iter().any(|arg| arg == "grok-4.7"));
        assert_eq!(arg_after(&launch.args, "--reasoning-effort"), "low");
        assert!(!launch.args.iter().any(|arg| arg == "-p"));
        assert_eq!(launch.stdin, None);
    }

    #[test]
    fn claude_prints_at_low_effort() {
        let launch = agent_launch(CaptionAgentId::Claude, "Edit this transcript.", "/tmp/job");
        assert_eq!(launch.command, "claude");
        assert!(launch.args.iter().any(|arg| arg == "claude-sonnet-5-5"));
        assert_eq!(arg_after(&launch.args, "--effort"), "low");
        assert_eq!(arg_after(&launch.args, "--tools"), "");
        assert!(!launch.args.iter().any(|arg| arg == "plan"));
        assert_eq!(
            launch.args.last().map(String::as_str),
            Some("Edit this transcript.")
        );
        assert_eq!(launch.stdin, None);
        assert!(!launch.stream_json);
    }

    #[test]
    fn sol_prints_without_writing_files() {
        let launch = agent_launch(CaptionAgentId::Sol, "Edit this transcript.", "/tmp/job");
        assert_eq!(launch.command, "codex");
        assert!(launch.args.iter().any(|arg| arg == "gpt-5.6-sol"));
        assert!(launch
            .args
            .iter()
            .any(|arg| arg == "model_reasoning_effort=\"low\""));
        assert!(launch.args.iter().any(|arg| arg == "read-only"));
        assert!(launch.args.iter().any(|arg| arg == "/tmp/job"));
        assert_eq!(launch.args.last().map(String::as_str), Some("-"));
        assert_eq!(launch.stdin.as_deref(), Some("Edit this transcript."));
    }

    #[test]
    fn agy_prints_at_low_effort() {
        let launch = agent_launch(CaptionAgentId::Agy, "Edit this transcript.", "/tmp/job");
        assert_eq!(launch.command, "agy");
        assert!(launch.args.iter().any(|arg| arg == "gemini-3.8-flash-low"));
        assert_eq!(arg_after(&launch.args, "--effort"), "low");
        assert_eq!(arg_after(&launch.args, "--print"), "Edit this transcript.");
        assert!(!launch.args.iter().any(|arg| arg == "plan"));
        assert_eq!(launch.stdin, None);
    }

    #[test]
    fn lists_only_installed_agents() {
        let only_grok = list_installed_agents(|command| {
            if command == "grok" {
                Some("/bin/grok".to_string())
            } else {
                None
            }
        });
        assert_eq!(
            only_grok,
            vec![CaptionAgentInfo {
                id: CaptionAgentId::Grok,
                label: "Grok 4.7"
            }]
        );

        let labels: Vec<&str> = list_installed_agents(|command| {
            if ["grok", "claude", "codex", "agy"].contains(&command) {
                Some(format!("/bin/{command}"))
            } else {
                None
            }
        })
        .iter()
        .map(|agent| agent.label)
        .collect();
        assert_eq!(
            labels,
            vec!["Grok 4.7", "Sonnet 5.5", "Sol 5.6", "Gemini 3.8 Flash"]
        );
    }

    #[test]
    fn ms_to_timecode_cases() {
        assert_eq!(ms_to_timecode(0), "00:00:00");
        assert_eq!(ms_to_timecode(5000), "00:00:05");
        assert_eq!(ms_to_timecode(90000), "00:01:30");
        assert_eq!(ms_to_timecode(3661000), "01:01:01");
        assert_eq!(ms_to_timecode(5999), "00:00:05");
    }

    #[test]
    fn format_transcript_for_llm_cases() {
        let result =
            format_transcript_for_llm(&[seg(0, 5000, "Hello"), seg(5000, 10000, "World")], 0);
        assert_eq!(
            result,
            "[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello\n[#1 00:00:05 -> 00:00:10 | 5000 -> 10000] World"
        );
        assert_eq!(format_transcript_for_llm(&[], 0), "");
        assert_eq!(
            format_transcript_for_llm(&[seg(0, 1000, "  spaced  ")], 0),
            "[#0 00:00:00 -> 00:00:01 | 0 -> 1000] spaced"
        );
        assert!(format_transcript_for_llm(&[seg(0, 1000, "A")], 3).starts_with("[#3 "));
    }

    #[test]
    fn parse_llm_response_clean_array() {
        let text = serde_json::to_string(&json!([
            {"title": "Clip 1", "start_ms": 0, "end_ms": 30000},
            {"title": "Clip 2", "start_ms": 60000, "end_ms": 120000}
        ]))
        .unwrap();
        assert_eq!(
            parse_llm_response(&text, None).unwrap(),
            vec![
                clip("Clip 1", 0, 30000, None),
                clip("Clip 2", 60000, 120000, None)
            ]
        );
    }

    #[test]
    fn parse_llm_response_markdown_fence() {
        let text = "Here are the clips:\n```json\n[{\"title\": \"Great moment\", \"start_ms\": 1000, \"end_ms\": 5000}]\n```";
        assert_eq!(
            parse_llm_response(text, None).unwrap(),
            vec![clip("Great moment", 1000, 5000, None)]
        );
    }

    #[test]
    fn parse_llm_response_surrounding_text() {
        let text = "Based on the transcript, here are the best clips:\n[{\"title\": \"Test\", \"start_ms\": 0, \"end_ms\": 1000}]\nHope that helps!";
        let result = parse_llm_response(text, None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Test");
    }

    #[test]
    fn parse_llm_response_missing_json() {
        let err = parse_llm_response("No clips found", None).unwrap_err();
        assert!(err.to_string().contains("No JSON array found"));
    }

    #[test]
    fn parse_llm_response_ids() {
        let segments = vec![
            seg(0, 5000, "A"),
            seg(5000, 12000, "B"),
            seg(12000, 20000, "C"),
        ];
        let text =
            serde_json::to_string(&json!([{"title": "From ids", "start_id": 1, "end_id": 2}]))
                .unwrap();
        assert_eq!(
            parse_llm_response(&text, Some(&segments)).unwrap(),
            vec![clip("From ids", 5000, 20000, None)]
        );
    }

    #[test]
    fn parse_llm_response_string_milliseconds() {
        let text = serde_json::to_string(
            &json!([{"title": "Strings", "start_ms": "1000", "end_ms": "5000"}]),
        )
        .unwrap();
        let result = parse_llm_response(&text, None).unwrap();
        assert_eq!(result[0].start_ms, 1000);
        assert_eq!(result[0].end_ms, 5000);
    }

    #[test]
    fn parse_llm_response_uses_the_last_array() {
        let text = "Example: [{\"title\": \"Example\", \"start_ms\": 0, \"end_ms\": 1}]\nHere are the clips:\n[{\"title\": \"Real\", \"start_ms\": 4000, \"end_ms\": 9000}]";
        let result = parse_llm_response(text, None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "Real");
    }

    #[test]
    fn parse_llm_response_category() {
        let text = serde_json::to_string(&json!([
            {"title": "Main topic clip", "start_ms": 0, "end_ms": 60000, "category": "related"},
            {"title": "Self-contained insight", "start_ms": 120000, "end_ms": 180000, "category": "standalone"}
        ]))
        .unwrap();
        assert_eq!(
            parse_llm_response(&text, None).unwrap(),
            vec![
                clip("Main topic clip", 0, 60000, Some(ClipCategory::Related)),
                clip(
                    "Self-contained insight",
                    120000,
                    180000,
                    Some(ClipCategory::Standalone)
                ),
            ]
        );
    }

    #[test]
    fn parse_llm_response_omits_missing_category() {
        let text = serde_json::to_string(
            &json!([{"title": "No category", "start_ms": 0, "end_ms": 30000}]),
        )
        .unwrap();
        assert_eq!(parse_llm_response(&text, None).unwrap()[0].category, None);
    }

    #[test]
    fn parse_llm_response_ignores_invalid_category() {
        let text = serde_json::to_string(&json!([
            {"title": "Bad category", "start_ms": 0, "end_ms": 30000, "category": "invalid"}
        ]))
        .unwrap();
        assert_eq!(parse_llm_response(&text, None).unwrap()[0].category, None);
    }

    #[test]
    fn parse_llm_response_swaps_inverted_times_and_default_title() {
        let text = r#"[{"end_ms": 1000, "start_ms": 5000}]"#;
        assert_eq!(
            parse_llm_response(text, None).unwrap(),
            vec![clip("Clip", 1000, 5000, None)]
        );
    }

    #[test]
    fn extract_json_array_fenced() {
        let text = "Sure.\n```json\n[{\"a\":1}]\n```\n";
        assert_eq!(extract_json_array(text).unwrap(), vec![json!({"a": 1})]);
    }

    #[test]
    fn extract_json_array_bracket_matching() {
        let text =
            "See [note] first. [{\"title\":\"Ok\",\"start_ms\":1,\"end_ms\":2}] trailing [x]";
        assert_eq!(
            extract_json_array(text).unwrap(),
            vec![json!({"title": "Ok", "start_ms": 1, "end_ms": 2})]
        );
    }

    #[test]
    fn extract_json_array_unwraps_clips_and_items() {
        assert_eq!(
            extract_json_array(r#"{"clips":[{"title":"A"}]}"#).unwrap(),
            vec![json!({"title": "A"})]
        );
        assert_eq!(
            extract_json_array(r#"{"items":[1,2]}"#).unwrap(),
            vec![json!(1), json!(2)]
        );
    }

    const LECTURE_ANSWER: &str = r#"[{"title":"Classe é a resposta do modelo","start_id":24,"end_id":37,"category":"standalone"},{"title":"Taxa, lote e épocas","start_id":132,"end_id":149,"category":"related"},{"title":"Imagem nova vira floresta","start_id":161,"end_id":182,"category":"standalone"},{"title":"Elefante sem classe confunde o modelo","start_id":201,"end_id":211,"category":"standalone"},{"title":"Mudou o dado, treine de novo","start_id":227,"end_id":254,"category":"standalone"},{"title":"Sem exemplos o modelo erra","start_id":255,"end_id":267,"category":"standalone"}]"#;

    const ELEPHANT_ANSWER: &str = r#"[{"title":"O elefante confunde o modelo","start_id":195,"end_id":211,"category":"standalone"}]"#;

    fn lecture() -> Vec<TranscriptSegment> {
        (0..280)
            .map(|i| seg(i * 1000, (i + 1) * 1000, &format!("line {i}")))
            .collect()
    }

    #[test]
    fn clips_from_agent_text_maps_the_lecture_answer() {
        let segments = lecture();
        let clips = clips_from_agent_text(LECTURE_ANSWER, &segments).unwrap();
        assert_eq!(
            clips
                .iter()
                .map(|clip| clip.title.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Classe é a resposta do modelo",
                "Taxa, lote e épocas",
                "Imagem nova vira floresta",
                "Elefante sem classe confunde o modelo",
                "Mudou o dado, treine de novo",
                "Sem exemplos o modelo erra",
            ]
        );
        assert_eq!(clips[0].start_ms, segments[24].start_ms);
        assert_eq!(clips[0].end_ms, segments[37].end_ms);
        assert_eq!(clips[3].category, Some(ClipCategory::Standalone));
        assert_eq!(clips[1].category, Some(ClipCategory::Related));
        assert!(clips
            .windows(2)
            .all(|pair| pair[1].start_ms >= pair[0].end_ms));
    }

    #[test]
    fn clips_from_agent_text_keeps_a_single_hinted_clip() {
        let segments = lecture();
        let clips = clips_from_agent_text(&format!("Note.\n{ELEPHANT_ANSWER}"), &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "O elefante confunde o modelo");
        assert_eq!(clips[0].start_ms, segments[195].start_ms);
        assert_eq!(clips[0].end_ms, segments[211].end_ms);
    }

    #[test]
    fn clips_from_agent_text_empty_array() {
        assert!(clips_from_agent_text("[]", &lecture()).unwrap().is_empty());
    }
}
