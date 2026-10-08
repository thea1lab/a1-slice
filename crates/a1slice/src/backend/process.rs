//! ffmpeg and ffprobe: probe a file, grab a frame, and run a command.

use a1slice_core::types::Ms;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

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

pub(super) fn run_ffmpeg(
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

pub(super) fn tail(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "ffmpeg failed".into();
    }
    let start = trimmed.len().saturating_sub(400);
    trimmed[start..].to_string()
}
