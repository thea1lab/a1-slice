//! Installed caption agents, and parsing the clip JSON they print.

use std::path::Path;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::types::{ClipCategory, ClipSegment, Ms, TranscriptSegment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionAgentId {
    Grok,
    Claude,
    Sol,
    Agy,
}

impl CaptionAgentId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Grok => "grok",
            Self::Claude => "claude",
            Self::Sol => "sol",
            Self::Agy => "agy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionAgent {
    pub id: CaptionAgentId,
    pub label: &'static str,
    pub command: &'static str,
}

pub const CAPTION_AGENTS: [CaptionAgent; 4] = [
    CaptionAgent {
        id: CaptionAgentId::Grok,
        label: "Grok 4.7",
        command: "grok",
    },
    CaptionAgent {
        id: CaptionAgentId::Claude,
        label: "Sonnet 5.5",
        command: "claude",
    },
    CaptionAgent {
        id: CaptionAgentId::Sol,
        label: "Sol 5.6",
        command: "codex",
    },
    CaptionAgent {
        id: CaptionAgentId::Agy,
        label: "Gemini 3.8 Flash",
        command: "agy",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionAgentInfo {
    pub id: CaptionAgentId,
    pub label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaunch {
    pub command: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub stream_json: bool,
}

/// `lookup` returns the resolved path, or `None` when the command is not installed.
/// An empty string counts as missing, matching a shell lookup that prints nothing.
pub fn command_on_path<F, S>(command: &str, lookup: F) -> Option<S>
where
    F: Fn(&str) -> Option<S>,
    S: AsRef<str>,
{
    lookup(command).filter(|found| !found.as_ref().is_empty())
}

pub fn list_installed_agents<F, S>(lookup: F) -> Vec<CaptionAgentInfo>
where
    F: Fn(&str) -> Option<S>,
    S: AsRef<str>,
{
    CAPTION_AGENTS
        .iter()
        .filter_map(|agent| {
            command_on_path(agent.command, &lookup)?;
            Some(CaptionAgentInfo {
                id: agent.id,
                label: agent.label,
            })
        })
        .collect()
}

pub fn agent_launch(id: CaptionAgentId, prompt: &str, work_dir: &str) -> AgentLaunch {
    match id {
        CaptionAgentId::Grok => {
            let mut args = vec![
                "--no-plan".into(),
                "--model".into(),
                "grok-4.7".into(),
                "--reasoning-effort".into(),
                "low".into(),
                "--output-format".into(),
                "plain".into(),
                "--no-subagents".into(),
                "--disable-web-search".into(),
                "--cwd".into(),
                work_dir.into(),
            ];
            if js_len(prompt) < 100_000 {
                args.insert(0, prompt.to_string());
                args.insert(0, "-p".into());
            } else {
                let instructions = Path::new(work_dir).join("instructions.md");
                args.insert(0, instructions.to_string_lossy().into_owned());
                args.insert(0, "--prompt-file".into());
            }
            AgentLaunch {
                command: "grok".into(),
                args,
                stdin: None,
                stream_json: false,
            }
        }
        CaptionAgentId::Claude => AgentLaunch {
            command: "claude".into(),
            args: vec![
                "-p".into(),
                "--bare".into(),
                "--tools".into(),
                String::new(),
                "--no-session-persistence".into(),
                "--model".into(),
                "claude-sonnet-5-5".into(),
                "--effort".into(),
                "low".into(),
                "--output-format".into(),
                "text".into(),
                prompt.into(),
            ],
            stdin: None,
            stream_json: false,
        },
        CaptionAgentId::Sol => AgentLaunch {
            command: "codex".into(),
            args: vec![
                "exec".into(),
                "--skip-git-repo-check".into(),
                "--ephemeral".into(),
                "--color".into(),
                "never".into(),
                "-m".into(),
                "gpt-5.6-sol".into(),
                "-c".into(),
                "model_reasoning_effort=\"low\"".into(),
                "-s".into(),
                "read-only".into(),
                "-C".into(),
                work_dir.into(),
                "-".into(),
            ],
            stdin: Some(prompt.to_string()),
            stream_json: false,
        },
        CaptionAgentId::Agy => AgentLaunch {
            command: "agy".into(),
            args: vec![
                "--model".into(),
                "gemini-3.8-flash-low".into(),
                "--effort".into(),
                "low".into(),
                "--output-format".into(),
                "text".into(),
                "--disable-slash-commands".into(),
                "--print".into(),
                prompt.into(),
            ],
            stdin: None,
            stream_json: false,
        },
    }
}

fn js_len(text: &str) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;
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
