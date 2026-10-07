//! Runs ffmpeg, whisper-cli, and the installed agents.
//!
//! The window sends work here and reads progress back on a channel.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use a1slice_core::clip_find::clip_find_prompt;
use a1slice_core::clip_find::format_clip_transcript;
use a1slice_core::clips::refine_clip_bounds;
use a1slice_core::crop::ffmpeg_crop_filter;
use a1slice_core::sidecars::{self, generate_srt, shift_subtitles};
use a1slice_core::types::{
    CaptionFont, CaptionLook, CaptionStyle, ClipCrop, ClipSegment, CropRatio, Ms, PipelineStage,
    ProgressUpdate, TranscriptSegment, VideoLanguage,
};

pub struct Running {
    pub cancel: Arc<AtomicBool>,
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub handle: JoinHandle<Result<JobDone, String>>,
}

#[derive(Debug, Clone)]
pub enum JobDone {
    Transcript(Vec<TranscriptSegment>),
    Clips {
        clips: Vec<ClipSegment>,
        raw: String,
    },
    CaptionLines(Vec<TranscriptSegment>),
    Folder(PathBuf),
}

pub fn list_agents() -> Vec<(&'static str, &'static str, &'static str)> {
    [
        ("grok", "Grok 4.7", "grok"),
        ("claude", "Sonnet 5.5", "claude"),
        ("sol", "Sol 5.6", "codex"),
        ("agy", "Gemini 3.8 Flash", "agy"),
    ]
    .into_iter()
    .filter(|(_, _, cmd)| which(cmd).is_some())
    .collect()
}

pub fn which(cmd: &str) -> Option<PathBuf> {
    let finder = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(finder).arg(cmd).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(PathBuf::from)
}

fn whisper_names() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("whisper-cli-win-x64.exe", "whisper-cli-win-x64.exe")
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            ("whisper-cli-mac-arm64-gpu", "whisper-cli-mac-arm64")
        } else {
            ("whisper-cli-mac-x64-gpu", "whisper-cli-mac-x64")
        }
    } else if cfg!(target_arch = "aarch64") {
        ("whisper-cli-linux-arm64", "whisper-cli-linux-arm64")
    } else {
        ("whisper-cli-linux-x64", "whisper-cli-linux-x64")
    }
}

pub fn whisper_binary() -> Option<PathBuf> {
    let (gpu, cpu) = whisper_names();
    let roots = [
        PathBuf::from("resources/bin"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/bin"),
    ];
    for name in [gpu, cpu] {
        for root in &roots {
            let path = root.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub fn probe(video: &Path) -> Result<(Ms, u32, u32), String> {
    let duration = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(video)
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&duration.stdout);
    let seconds: f64 = text
        .trim()
        .parse()
        .map_err(|_| format!("Could not read the length of {}", video.display()))?;
    let size = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(video)
        .output()
        .map_err(|e| e.to_string())?;
    let size_text = String::from_utf8_lossy(&size.stdout);
    let mut parts = size_text.trim().split('x');
    let width: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let height: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok(((seconds * 1000.0).round() as Ms, width, height))
}

pub fn grab_frame(video: &Path, at_ms: Ms, max_w: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let (_, width, height) = probe(video)?;
    if width == 0 || height == 0 {
        return Err("This file has no picture.".into());
    }
    let scale_w = max_w.min(width).max(2) / 2 * 2;
    let scale_h = ((height as f64) * (scale_w as f64) / (width as f64))
        .round()
        .max(2.0) as u32
        / 2
        * 2;
    let sec = format!("{:.3}", (at_ms.max(0) as f64) / 1000.0);
    let output = Command::new("ffmpeg")
        .args(["-ss", &sec, "-i"])
        .arg(video)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-vf",
            &format!("scale={scale_w}:{scale_h}"),
            "-v",
            "error",
            "pipe:1",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    let expected = scale_w as usize * scale_h as usize * 4;
    if output.stdout.len() < expected {
        return Err("Could not read a frame from this video.".into());
    }
    Ok((scale_w, scale_h, output.stdout[..expected].to_vec()))
}

fn run_ffmpeg(
    args: &[String],
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped.".into());
    }
    let mut child = Command::new("ffmpeg")
        .args(args.iter().map(String::as_str))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg is not available ({e})"))?;
    let mut stderr = child.stderr.take();
    {
        let mut slot = child_slot.lock().unwrap();
        *slot = Some(child);
    }
    let mut err_text = String::new();
    if let Some(pipe) = stderr.as_mut() {
        let _ = pipe.read_to_string(&mut err_text);
    }
    let status = {
        let mut slot = child_slot.lock().unwrap();
        let child = slot.take();
        child
            .map(|mut c| c.wait())
            .transpose()
            .map_err(|e| e.to_string())?
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped.".into());
    }
    match status {
        Some(code) if code.success() => Ok(()),
        _ => Err(tail(&err_text)),
    }
}

fn tail(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "ffmpeg failed".into();
    }
    let start = trimmed.len().saturating_sub(400);
    trimmed[start..].to_string()
}

fn cut_args(
    input: &Path,
    output: &Path,
    start_sec: f64,
    duration_sec: f64,
    filter: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "-ss".into(),
        format!("{start_sec}"),
        "-i".into(),
        input.display().to_string(),
        "-t".into(),
        format!("{duration_sec}"),
    ];
    if let Some(filter) = filter {
        args.push("-vf".into());
        args.push(filter.into());
    }
    args.extend([
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "18".into(),
        "-c:a".into(),
        "aac".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
        "-y".into(),
        output.display().to_string(),
    ]);
    args
}

fn copy_args(input: &Path, output: &Path, start_sec: f64, duration_sec: f64) -> Vec<String> {
    vec![
        "-ss".into(),
        format!("{start_sec}"),
        "-i".into(),
        input.display().to_string(),
        "-t".into(),
        format!("{duration_sec}"),
        "-c".into(),
        "copy".into(),
        "-avoid_negative_ts".into(),
        "make_zero".into(),
        "-y".into(),
        output.display().to_string(),
    ]
}

pub fn export_kept_clips(
    video: &Path,
    clips: &[ClipSegment],
    segments: &[TranscriptSegment],
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
    on_progress: impl Fn(ProgressUpdate),
) -> Result<PathBuf, String> {
    let (dir, stem) = sidecars::video_stem(video);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_dir = dir.join(format!("a1slice-{stem}-{stamp}"));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    std::fs::write(
        out_dir.join("transcript.txt"),
        format_clip_transcript(segments),
    )
    .ok();
    let duration = probe(video).ok().map(|(ms, _, _)| ms);
    let size = probe(video).ok().map(|(_, w, h)| (w, h));
    if clips.is_empty() {
        return Err("Keep at least one clip.".into());
    }
    for (i, clip) in clips.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Stopped.".into());
        }
        let start = duration
            .map(|d| clip.start_ms.min(d))
            .unwrap_or(clip.start_ms);
        let end = duration.map(|d| clip.end_ms.min(d)).unwrap_or(clip.end_ms);
        if end <= start {
            continue;
        }
        let safe: String = clip
            .title
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
            .take(50)
            .collect();
        let file = out_dir.join(format!("{:02}_{safe}.mp4", i + 1));
        on_progress(ProgressUpdate {
            stage: PipelineStage::Cutting,
            message: format!("Cutting clip {}/{}: {}", i + 1, clips.len(), clip.title),
            percent: ((i + 1) as f64 / clips.len() as f64) * 100.0,
        });
        let shifted = shift_subtitles(segments, start, end);
        let srt_path = file.with_extension("srt");
        std::fs::write(&srt_path, generate_srt(&shifted)).map_err(|e| e.to_string())?;
        let filter = clip
            .crop
            .and_then(|crop| size.and_then(|(w, h)| ffmpeg_crop_filter(crop, w as f64, h as f64)));
        let start_sec = start as f64 / 1000.0;
        let duration_sec = ((end - start) as f64 / 1000.0).max(0.2);
        if run_ffmpeg(
            &cut_args(video, &file, start_sec, duration_sec, filter.as_deref()),
            cancel,
            child_slot,
        )
        .is_err()
        {
            run_ffmpeg(
                &copy_args(video, &file, start_sec, duration_sec),
                cancel,
                child_slot,
            )?;
        }
    }
    Ok(out_dir)
}

pub fn export_reframe(
    video: &Path,
    crop: ClipCrop,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<PathBuf, String> {
    let (duration, w, h) = probe(video)?;
    let (dir, stem) = sidecars::video_stem(video);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_dir = dir.join(format!("a1slice-{stem}-reframe-{stamp}"));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let file = out_dir.join(format!("{stem}.mp4"));
    let filter = if crop.ratio == CropRatio::Original {
        None
    } else {
        ffmpeg_crop_filter(crop, w as f64, h as f64)
    };
    let duration_sec = (duration as f64 / 1000.0).max(0.2);
    run_ffmpeg(
        &cut_args(video, &file, 0.0, duration_sec, filter.as_deref()),
        cancel,
        child_slot,
    )?;
    Ok(out_dir)
}

pub fn export_captions(
    video: &Path,
    cues: &[TranscriptSegment],
    look: CaptionLook,
    style: &CaptionStyle,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<PathBuf, String> {
    let (dir, stem) = sidecars::video_stem(video);
    if look == CaptionLook::Srt {
        let path = dir.join(format!("{stem}.srt"));
        std::fs::write(&path, generate_srt(cues)).map_err(|e| e.to_string())?;
        return Ok(path);
    }
    let (duration, _, _) = probe(video)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_dir = dir.join(format!("a1slice-{stem}-captions-{stamp}"));
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let file = out_dir.join(format!("{stem}.mp4"));
    let srt = std::env::temp_dir().join(format!("a1slice-{stamp}.srt"));
    std::fs::write(&srt, generate_srt(cues)).map_err(|e| e.to_string())?;
    let font = find_font(style.font)
        .ok_or_else(|| "No caption font found on this computer".to_string())?;
    let filter = caption_burn_filter(&srt, &font.0, &font.1, style);
    let duration_sec = (duration as f64 / 1000.0).max(0.2);
    let result = run_ffmpeg(
        &cut_args(video, &file, 0.0, duration_sec, Some(&filter)),
        cancel,
        child_slot,
    );
    let _ = std::fs::remove_file(&srt);
    result?;
    Ok(out_dir)
}

fn find_font(font: CaptionFont) -> Option<(PathBuf, String)> {
    let candidates: &[(&str, &str)] = match font {
        CaptionFont::Serif => &[
            (
                "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf",
                "Noto Serif",
            ),
            ("/usr/share/fonts/noto/NotoSerif-Regular.ttf", "Noto Serif"),
            (
                "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
                "Liberation Serif",
            ),
            (
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
                "DejaVu Serif",
            ),
        ],
        CaptionFont::Mono => &[
            (
                "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
                "Noto Sans Mono",
            ),
            (
                "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
                "Noto Sans Mono",
            ),
            (
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "DejaVu Sans Mono",
            ),
        ],
        CaptionFont::Sans => &[
            (
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                "Noto Sans",
            ),
            ("/usr/share/fonts/noto/NotoSans-Regular.ttf", "Noto Sans"),
            (
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                "Liberation Sans",
            ),
            (
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "DejaVu Sans",
            ),
        ],
    };
    candidates
        .iter()
        .find(|(path, _)| Path::new(path).is_file())
        .map(|(p, n)| (PathBuf::from(p), (*n).to_string()))
}

fn caption_burn_filter(
    srt: &Path,
    fonts_dir: &Path,
    font_name: &str,
    style: &CaptionStyle,
) -> String {
    let (ass, outline, size, margin, align) = style_force(style);
    let force = format!(
        "FontName={font_name}\\,FontSize={size}\\,PrimaryColour={ass}\\,OutlineColour={outline}\\,BorderStyle=1\\,Outline=0.55\\,Shadow=0\\,Bold=0\\,Alignment={align}\\,MarginV={margin}"
    );
    let srt = escape_filter(srt);
    let fonts = escape_filter(fonts_dir);
    format!("subtitles='{srt}':fontsdir='{fonts}':force_style='{force}'")
}

fn style_force(style: &CaptionStyle) -> (String, String, i64, i64, i32) {
    let (ass, outline) = if let Some(hex) = style.custom_color.as_deref().and_then(hex_to_ass) {
        (hex, "&H00000000".to_string())
    } else {
        match style.color {
            a1slice_core::types::CaptionColor::Cream => ("&H00E0F8FF".into(), "&H00000000".into()),
            a1slice_core::types::CaptionColor::Yellow => ("&H004AE1FF".into(), "&H00000000".into()),
            a1slice_core::types::CaptionColor::Black => ("&H00111111".into(), "&H00FFFFFF".into()),
            a1slice_core::types::CaptionColor::White => ("&H00FFFFFF".into(), "&H00000000".into()),
        }
    };
    let (preset, margin) = match style.size {
        a1slice_core::types::CaptionSize::Small => (18, 36),
        a1slice_core::types::CaptionSize::Medium => (24, 60),
        a1slice_core::types::CaptionSize::Large => (28, 90),
    };
    let size = style.font_size.unwrap_or(preset).clamp(8, 96);
    let align = match style.position {
        a1slice_core::types::CaptionPosition::Top => 8,
        a1slice_core::types::CaptionPosition::Middle => 5,
        a1slice_core::types::CaptionPosition::Bottom => 2,
    };
    let margin = if matches!(style.position, a1slice_core::types::CaptionPosition::Middle) {
        0
    } else {
        margin
    };
    (ass, outline, size, margin, align)
}

fn hex_to_ass(hex: &str) -> Option<String> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(format!("&H00{b:02X}{g:02X}{r:02X}"))
}

fn escape_filter(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

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
    instruction: &str,
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
) -> Result<Vec<TranscriptSegment>, String> {
    let body = segments
        .iter()
        .map(|s| s.text.trim())
        .collect::<Vec<_>>()
        .join("\n");
    let prompt = format!(
        "Rewrite this transcript. {instruction}\n\nKeep the same number of lines and the same order. Print only the new transcript, one line per cue.\n\n{body}"
    );
    let raw = run_agent(agent_id, &prompt, cancel, child_slot)?;
    if raw.trim().eq_ignore_ascii_case("none") {
        return Ok(segments.to_vec());
    }
    let lines: Vec<&str> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() != segments.len() {
        return Err("The edit changed how many lines there are, so it was not applied.".into());
    }
    Ok(segments
        .iter()
        .zip(lines)
        .map(|(seg, text)| TranscriptSegment {
            text: text.to_string(),
            ..seg.clone()
        })
        .collect())
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

fn clips_from_text(text: &str, segments: &[TranscriptSegment]) -> Result<Vec<ClipSegment>, String> {
    let arrays = extract_arrays(text);
    if arrays.is_empty() {
        return Err("The agent did not return a clip list.".into());
    }
    let mut found = Vec::new();
    for array in arrays {
        let clips = clips_from_array(&array, segments);
        if !clips.is_empty() {
            found = clips;
        }
    }
    Ok(found)
}

fn clips_from_array(
    array: &[serde_json::Value],
    segments: &[TranscriptSegment],
) -> Vec<ClipSegment> {
    let mut clips = Vec::new();
    for item in array {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Clip")
            .to_string();
        let start_id = obj
            .get("start_id")
            .or_else(|| obj.get("startId"))
            .and_then(json_index);
        let end_id = obj
            .get("end_id")
            .or_else(|| obj.get("endId"))
            .and_then(json_index);
        let (start_ms, end_ms) = if let (Some(start_id), Some(end_id)) = (start_id, end_id) {
            match (segments.get(start_id), segments.get(end_id)) {
                (Some(start), Some(end)) => (start.start_ms, end.end_ms),
                _ => continue,
            }
        } else {
            let Some(start_ms) = obj
                .get("start_ms")
                .or_else(|| obj.get("startMs"))
                .and_then(json_ms)
            else {
                continue;
            };
            let Some(end_ms) = obj
                .get("end_ms")
                .or_else(|| obj.get("endMs"))
                .and_then(json_ms)
            else {
                continue;
            };
            (start_ms, end_ms)
        };
        let (start_ms, end_ms) = if end_ms < start_ms {
            (end_ms, start_ms)
        } else {
            (start_ms, end_ms)
        };
        let category = match obj.get("category").and_then(|v| v.as_str()) {
            Some("related") => Some(a1slice_core::types::ClipCategory::Related),
            Some("standalone") => Some(a1slice_core::types::ClipCategory::Standalone),
            _ => None,
        };
        let clip = ClipSegment {
            title,
            start_ms: start_ms + 1,
            end_ms,
            category,
            topic: None,
            crop: None,
            approved: Some(true),
        };
        let refined = refine_clip_bounds(clip, segments);
        if refined.end_ms > refined.start_ms {
            clips.push(refined);
        }
    }
    clips
}

fn json_index(value: &serde_json::Value) -> Option<usize> {
    json_ms(value).filter(|n| *n >= 0).map(|n| n as usize)
}

fn json_ms(value: &serde_json::Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    if let Some(n) = value.as_u64() {
        return i64::try_from(n).ok();
    }
    if let Some(n) = value.as_f64() {
        if n.is_finite() {
            return Some(n.round() as i64);
        }
    }
    let text = value.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(n) = text.parse::<i64>() {
        return Some(n);
    }
    text.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(|n| n.round() as i64)
}

fn extract_arrays(text: &str) -> Vec<Vec<serde_json::Value>> {
    let fenced = text.split("```").nth(1).map(|block| {
        block
            .trim_start_matches("json")
            .trim_start_matches("JSON")
            .trim()
    });
    let owned;
    let sources: Vec<&str> = if let Some(block) = fenced {
        owned = block.to_string();
        vec![owned.as_str(), text]
    } else {
        vec![text]
    };
    for src in sources {
        let found = arrays_in(src);
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

fn arrays_in(src: &str) -> Vec<Vec<serde_json::Value>> {
    let bytes = src.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' || bytes[i] == b'{' {
            if let Some(slice) = balanced(src, i) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(slice) {
                    if let Some(array) = unwrap_array(&value) {
                        found.push(array);
                        i += slice.len();
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    found
}

fn balanced(text: &str, start: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let open = bytes.get(start).copied()?;
    let close = if open == b'[' { b']' } else { b'}' };
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;
    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            if escape {
                escape = false;
            } else if ch == b'\\' {
                escape = true;
            } else if ch == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if ch == b'"' {
            in_string = true;
            i += 1;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(&text[start..=i]);
            }
        }
        i += 1;
    }
    None
}

fn unwrap_array(value: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
    if let Some(array) = value.as_array() {
        return Some(array.clone());
    }
    let obj = value.as_object()?;
    for key in [
        "clips",
        "items",
        "hooks",
        "candidates",
        "segments",
        "topics",
    ] {
        if let Some(array) = obj.get(key).and_then(|v| v.as_array()) {
            return Some(array.clone());
        }
    }
    let arrays: Vec<_> = obj.values().filter_map(|v| v.as_array().cloned()).collect();
    if arrays.len() == 1 {
        return Some(arrays.into_iter().next().unwrap());
    }
    None
}

pub fn open_path(path: &Path) -> Result<(), String> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn spawn_job<F>(work: F) -> Running
where
    F: FnOnce(Arc<AtomicBool>, Arc<Mutex<Option<std::process::Child>>>) -> Result<JobDone, String>
        + Send
        + 'static,
{
    let cancel = Arc::new(AtomicBool::new(false));
    let child = Arc::new(Mutex::new(None));
    let cancel_thread = Arc::clone(&cancel);
    let child_thread = Arc::clone(&child);
    let handle = std::thread::spawn(move || work(cancel_thread, child_thread));
    Running {
        cancel,
        child,
        handle,
    }
}

pub fn cancel_job(running: &Running) {
    running.cancel.store(true, Ordering::Relaxed);
    if let Some(mut child) = running.child.lock().unwrap().take() {
        let _ = child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_text_becomes_a_clip_snapped_to_the_transcript() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 4000,
                text: "Hello".into(),
            },
            TranscriptSegment {
                start_ms: 4500,
                end_ms: 9000,
                text: "World".into(),
            },
        ];
        let text = "Here you go:\n```json\n[{\"title\":\"Hook\",\"start_id\":0,\"end_id\":1,\"category\":\"standalone\"}]\n```";
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Hook");
        assert!(clips[0].end_ms > clips[0].start_ms);
    }

    #[test]
    fn later_clip_list_wins_over_the_example_in_the_prompt() {
        let segments = vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 4000,
                text: "Hello".into(),
            },
            TranscriptSegment {
                start_ms: 4500,
                end_ms: 9000,
                text: "World".into(),
            },
        ];
        let text = r#"Example: [{"title":"short title","start_id":10,"end_id":18}]
[{"title":"Hook","start_id":"0","end_id":"1","category":"standalone"}]"#;
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Hook");
    }

    #[test]
    fn wrapped_clips_object_is_read() {
        let segments = vec![TranscriptSegment {
            start_ms: 1000,
            end_ms: 5000,
            text: "Line".into(),
        }];
        let text = r#"{"clips":[{"title":"Bit","start_ms":1000,"end_ms":5000}]}"#;
        let clips = clips_from_text(text, &segments).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].title, "Bit");
    }

    #[test]
    fn whisper_timestamp_parses() {
        assert_eq!(parse_timestamp("01:02:03.004"), 3_723_004);
        assert_eq!(parse_timestamp("00:00:01,500"), 1500);
    }

    #[test]
    #[ignore = "needs ffmpeg and ffprobe"]
    fn generated_video_frame_reframe_and_caption_export() {
        let dir = std::env::temp_dir().join(format!("a1slice-it-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("clip.mp4");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=320x240:d=1",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=320x240:d=1",
                "-filter_complex",
                "[0:v][1:v]concat=n=2:v=1:a=0",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=2",
                "-shortest",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-y",
            ])
            .arg(&src)
            .status()
            .expect("ffmpeg");
        assert!(status.success(), "ffmpeg could not build the fixture");
        let (ms, w, h) = probe(&src).unwrap();
        assert!((ms - 2000).abs() < 400, "duration {ms}");
        assert_eq!((w, h), (320, 240));

        let (_fw, _fh, red) = grab_frame(&src, 200, 320).unwrap();
        assert!(
            red[0] > 180 && red[1] < 40,
            "expected a red frame, got {:?}",
            &red[..4]
        );
        let (_fw, _fh, blue) = grab_frame(&src, 1500, 320).unwrap();
        assert!(
            blue[2] > 180 && blue[0] < 40,
            "expected a blue frame, got {:?}",
            &blue[..4]
        );

        let cancel = std::sync::atomic::AtomicBool::new(false);
        let slot = std::sync::Mutex::new(None);
        let crop = a1slice_core::types::ClipCrop {
            ratio: CropRatio::Square,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        };
        let out = export_reframe(&src, crop, &cancel, &slot).unwrap();
        let mp4 = std::fs::read_dir(&out)
            .unwrap()
            .find_map(|e| e.ok())
            .unwrap()
            .path();
        let (_ms, ow, oh) = probe(&mp4).unwrap();
        assert_eq!(
            ow, oh,
            "square export should have equal sides, got {ow}x{oh}"
        );

        let cues = vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "Hello".into(),
        }];
        let style = a1slice_core::types::CaptionStyle {
            color: a1slice_core::types::CaptionColor::White,
            custom_color: None,
            position: a1slice_core::types::CaptionPosition::Bottom,
            size: a1slice_core::types::CaptionSize::Large,
            font_size: None,
            font: CaptionFont::Sans,
        };
        let burned =
            export_captions(&src, &cues, CaptionLook::Burn, &style, &cancel, &slot).unwrap();
        assert!(burned.is_dir());
        let srt = export_captions(&src, &cues, CaptionLook::Srt, &style, &cancel, &slot).unwrap();
        let text = std::fs::read_to_string(&srt).unwrap();
        assert!(text.contains("Hello"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
