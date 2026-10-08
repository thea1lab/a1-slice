//! Run an installed agent for clip finding and caption fixes.

use a1slice_core::caption_edit::{
    apply_edited_caption_text, caption_edit_prompt, caption_file_text, extract_printed_caption,
    is_none_answer, CaptionEditRequest, CaptionEditResult,
};
use a1slice_core::clip_find::clip_find_prompt;
use a1slice_core::types::{ClipSegment, TranscriptSegment};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use super::clip_parse::clips_from_text;
use super::process::{tail, which};

pub fn find_clips(
    segments: &[TranscriptSegment],
    agent_id: &str,
    hint: &str,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<(Vec<ClipSegment>, String), String> {
    let prompt = clip_find_prompt(segments, Some(hint));
    let raw = run_agent(agent_id, &prompt, cancel, child_slot)?;
    let clips = clips_from_text(&raw, segments)?;
    if clips.is_empty() {
        return Err("The agent did not return any clips.".into());
    }
    Ok((clips, raw))
}

pub fn fix_words(
    segments: &[TranscriptSegment],
    agent_id: &str,
    request: &CaptionEditRequest,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<Vec<TranscriptSegment>, String> {
    let prompt = caption_edit_prompt(request, &caption_file_text(segments));
    let raw = run_agent(agent_id, &prompt, cancel, child_slot)?;
    lines_from_agent_output(segments, &raw)
}

/// Read the transcript the agent printed. `NONE` keeps the lines that were sent.
pub fn lines_from_agent_output(
    before: &[TranscriptSegment],
    output: &str,
) -> Result<Vec<TranscriptSegment>, String> {
    if is_none_answer(output) {
        return Ok(before.to_vec());
    }
    let Some(printed) = extract_printed_caption(output) else {
        return Err(
            "The agent did not return the edited lines. The lines were left as they are.".into(),
        );
    };
    if normalize_caption_file(&printed) == normalize_caption_file(&caption_file_text(before)) {
        return Ok(before.to_vec());
    }
    match apply_edited_caption_text(before, &printed) {
        CaptionEditResult::Segments(lines) => Ok(lines),
        CaptionEditResult::Error(message) => Err(message),
    }
}

fn normalize_caption_file(text: &str) -> String {
    text.replace("\r\n", "\n").trim().to_string()
}

fn run_agent(
    agent_id: &str,
    prompt: &str,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<String, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped.".into());
    }
    let dir = std::env::temp_dir().join(format!("a1-agent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    std::fs::write(dir.join("instructions.md"), prompt).ok();
    let (cmd, mut args, stdin_prompt) = agent_command(agent_id, prompt, &dir);
    if which(cmd).is_none() {
        return Err(format!("{cmd} is not installed."));
    }
    if prompt.len() >= 100_000 && cmd == "grok" {
        args = agent_command(agent_id, prompt, &dir).1;
    }
    let mut command = Command::new(cmd);
    command
        .args(&args)
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin_prompt {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    if stdin_prompt {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(prompt.as_bytes());
        }
    }
    {
        let mut slot = child_slot.lock().unwrap();
        *slot = Some(child);
    }
    let mut slot = child_slot.lock().unwrap();
    let mut child = slot
        .take()
        .ok_or_else(|| "The agent did not start.".to_string())?;
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut out);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut err);
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped.".into());
    }
    if !status.success() && out.trim().is_empty() {
        return Err(if err.trim().is_empty() {
            "The agent stopped.".into()
        } else {
            tail(&err)
        });
    }
    if out.trim().is_empty() {
        Ok(err)
    } else {
        Ok(out)
    }
}

fn agent_command(id: &str, prompt: &str, work_dir: &Path) -> (&'static str, Vec<String>, bool) {
    match id {
        "claude" => (
            "claude",
            vec![
                "-p".into(),
                "--bare".into(),
                "--tools".into(),
                "".into(),
                "--no-session-persistence".into(),
                "--model".into(),
                "claude-sonnet-5-5".into(),
                "--effort".into(),
                "low".into(),
                "--output-format".into(),
                "text".into(),
                prompt.into(),
            ],
            false,
        ),
        "sol" => (
            "codex",
            vec![
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
                work_dir.display().to_string(),
                "-".into(),
            ],
            true,
        ),
        "agy" => (
            "agy",
            vec![
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
            false,
        ),
        _ => {
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
                work_dir.display().to_string(),
            ];
            if prompt.len() < 100_000 {
                args.insert(0, prompt.into());
                args.insert(0, "-p".into());
            } else {
                args.insert(0, work_dir.join("instructions.md").display().to_string());
                args.insert(0, "--prompt-file".into());
            }
            ("grok", args, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::lines_from_agent_output;
    use a1slice_core::types::TranscriptSegment;

    fn seg(start_ms: i64, end_ms: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.to_string(),
        }
    }

    fn before() -> Vec<TranscriptSegment> {
        vec![
            seg(0, 2000, "Hello teh world"),
            seg(2000, 6000, "This line is much too long for one caption"),
        ]
    }

    #[test]
    fn none_keeps_the_lines_that_were_sent() {
        assert_eq!(
            lines_from_agent_output(&before(), "NONE").unwrap(),
            before()
        );
    }

    #[test]
    fn a_printed_transcript_can_fix_a_typo_without_keeping_the_line_count() {
        let output = "0:00  Hello the world\n0:02  This line is much\ntoo long for one caption\n";
        let lines = lines_from_agent_output(&before(), output).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "Hello the world");
        assert_eq!(lines[2].text, "too long for one caption");
    }

    #[test]
    fn an_answer_without_caption_lines_is_refused() {
        let error = lines_from_agent_output(&before(), "I rewrote it.").unwrap_err();
        assert!(error.contains("did not return"));
    }
}
