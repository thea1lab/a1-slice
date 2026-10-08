//! whisper-cli and the JSON it prints.

use a1slice_core::sidecars::{self};
use a1slice_core::types::{Ms, PipelineStage, ProgressUpdate, TranscriptSegment, VideoLanguage};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use super::process::{run_ffmpeg, tail, whisper_binary};

pub fn transcribe_video(
    video: &Path,
    language: VideoLanguage,
    entropy: f64,
    max_context: i64,
    beam: i64,
    temperature: f64,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
    on_progress: impl Fn(ProgressUpdate),
) -> Result<Vec<TranscriptSegment>, String> {
    let binary = whisper_binary().ok_or_else(|| {
        "The whisper program is not built yet. Run scripts/build-whisper.sh, then try again."
            .to_string()
    })?;
    let model = sidecars::model_path();
    if !model.is_file() {
        return Err("The Whisper model is not in ~/.a1slice/models yet.".into());
    }
    on_progress(ProgressUpdate {
        stage: PipelineStage::Extracting,
        message: "Reading the audio…".into(),
        percent: 5.0,
    });
    let wav = std::env::temp_dir().join(format!("a1slice-{}.wav", std::process::id()));
    let extract = vec![
        "-i".into(),
        video.display().to_string(),
        "-vn".into(),
        "-ac".into(),
        "1".into(),
        "-ar".into(),
        "16000".into(),
        "-af".into(),
        "afftdn".into(),
        "-y".into(),
        wav.display().to_string(),
    ];
    run_ffmpeg(&extract, cancel, child_slot)?;
    on_progress(ProgressUpdate {
        stage: PipelineStage::Transcribing,
        message: "Writing the words…".into(),
        percent: 20.0,
    });
    let output_base = wav.with_extension("");
    let mut args = vec![
        "-m".into(),
        model.display().to_string(),
        "-f".into(),
        wav.display().to_string(),
        "-oj".into(),
        "-of".into(),
        output_base.display().to_string(),
        "-pp".into(),
        "--entropy-thold".into(),
        format!("{entropy}"),
        "--max-context".into(),
        format!("{max_context}"),
        "-bs".into(),
        format!("{beam}"),
        "-tpi".into(),
        format!("{temperature}"),
    ];
    if language != VideoLanguage::Auto {
        args.push("-l".into());
        args.push(language.as_str().into());
    }
    let child = Command::new(&binary)
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    {
        let mut slot = child_slot.lock().unwrap();
        *slot = Some(child);
    }
    // Re-take the child to wait. The slot holds it.
    let status = {
        let mut slot = child_slot.lock().unwrap();
        let mut child = slot
            .take()
            .ok_or_else(|| "Whisper did not start.".to_string())?;
        let mut err = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut err);
        }
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("Whisper stopped: {}", tail(&err)));
        }
        status
    };
    let _ = status;
    let json_path = PathBuf::from(format!("{}.json", output_base.display()));
    let json = std::fs::read_to_string(&json_path)
        .map_err(|_| "Whisper did not write a transcript.".to_string())?;
    let _ = std::fs::remove_file(&wav);
    let _ = std::fs::remove_file(&json_path);
    parse_whisper_json(&json)
}

fn parse_whisper_json(json: &str) -> Result<Vec<TranscriptSegment>, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let items = value
        .get("transcription")
        .and_then(|v| v.as_array())
        .ok_or("Whisper returned no transcript.")?;
    let mut segments = Vec::new();
    for item in items {
        let text = item
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        let from = item
            .pointer("/timestamps/from")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let to = item
            .pointer("/timestamps/to")
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let start_ms = parse_timestamp(from);
        let end_ms = parse_timestamp(to).max(start_ms + 1);
        segments.push(TranscriptSegment {
            start_ms,
            end_ms,
            text,
        });
    }
    Ok(segments)
}

fn parse_timestamp(ts: &str) -> Ms {
    let ts = ts.replace(',', ".");
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() < 3 {
        return 0;
    }
    let hours: i64 = parts[0].parse().unwrap_or(0);
    let minutes: i64 = parts[1].parse().unwrap_or(0);
    let sec_parts: Vec<&str> = parts[2].split('.').collect();
    let seconds: i64 = sec_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let frac = sec_parts.get(1).copied().unwrap_or("0");
    let millis: i64 = format!("{frac:0<3}")
        .chars()
        .take(3)
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    hours * 3_600_000 + minutes * 60_000 + seconds * 1000 + millis
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whisper_timestamp_parses() {
        assert_eq!(parse_timestamp("01:02:03.004"), 3_723_004);
        assert_eq!(parse_timestamp("00:00:01,500"), 1500);
    }
}
