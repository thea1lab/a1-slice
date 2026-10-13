//! ffmpeg and ffprobe: probe a file, grab a frame, and run a command.

use a1slice_core::types::Ms;
use std::io::{BufRead, BufReader, Read};
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

fn whisper_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("resources/bin"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/bin"),
    ];
    if let Ok(appdir) = std::env::var("APPDIR") {
        roots.insert(0, PathBuf::from(appdir).join("resources/bin"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.join("resources/bin"));
            if let Some(grandparent) = parent.parent() {
                roots.push(grandparent.join("resources/bin"));
            }
        }
    }
    roots
}

fn names_in(dir: &Path) -> Option<Vec<String>> {
    let entries = std::fs::read_dir(dir).ok()?;
    Some(
        entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect(),
    )
}

/// GPU binary when that file is in any search directory. CPU binary otherwise.
pub fn whisper_binary() -> Option<PathBuf> {
    let listed: Vec<(PathBuf, Vec<String>)> = whisper_roots()
        .into_iter()
        .filter_map(|root| {
            let names = names_in(&root)?;
            Some((root, names))
        })
        .collect();
    let borrowed: Vec<(&Path, Vec<&str>)> = listed
        .iter()
        .map(|(dir, names)| (dir.as_path(), names.iter().map(String::as_str).collect()))
        .collect();
    let locations: Vec<(&Path, &[&str])> = borrowed
        .iter()
        .map(|(dir, names)| (*dir, names.as_slice()))
        .collect();
    let fallback = listed.first().map(|(dir, _)| dir.clone())?;
    let chosen = a1slice_core::whisper::choose_whisper_binary_path(
        std::env::consts::OS,
        std::env::consts::ARCH,
        &locations,
        &fallback,
    );
    chosen.is_file().then_some(chosen)
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
    let ms = a1slice_core::ffmpeg_cmd::parse_ffprobe_duration(&text)
        .ok_or_else(|| format!("Could not read the length of {}", video.display()))?;
    let (width, height) = probe_size(video)?;
    Ok((ms, width, height))
}

pub fn probe_size(video: &Path) -> Result<(u32, u32), String> {
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
    let (width, height) =
        a1slice_core::ffmpeg_cmd::parse_ffprobe_dimensions(&size_text).unwrap_or((0, 0));
    if width == 0 || height == 0 {
        return Err("This file has no picture.".into());
    }
    Ok((width, height))
}

#[allow(dead_code)]
pub fn grab_frame(video: &Path, at_ms: Ms, max_w: u32) -> Result<(u32, u32, Vec<u8>), String> {
    grab_frame_sized(video, at_ms, max_w, None)
}

pub fn grab_frame_sized(
    video: &Path,
    at_ms: Ms,
    max_w: u32,
    known_size: Option<(u32, u32)>,
) -> Result<(u32, u32, Vec<u8>), String> {
    let (width, height) = match known_size {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        _ => probe_size(video)?,
    };
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

pub struct VideoStream {
    child: std::process::Child,
    pub frame_rx: std::sync::mpsc::Receiver<(u32, u32, Vec<u8>)>,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for VideoStream {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        #[cfg(windows)]
        {
            let pid = self.child.id();
            let mut kill_cmd = std::process::Command::new("taskkill");
            kill_cmd
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            {
                use std::os::windows::process::CommandExt;
                kill_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            }
            let _ = kill_cmd.status();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl VideoStream {
    pub fn stop(self) {
        // Drop handles cleanup
    }
}

pub fn start_video_stream(
    video: &Path,
    at_ms: Ms,
    max_w: u32,
    size: (u32, u32),
) -> Result<VideoStream, String> {
    let (width, height) = if size.0 > 0 && size.1 > 0 {
        size
    } else {
        probe_size(video)?
    };
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
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-re", "-ss", &sec, "-i"])
        .arg(video)
        .args([
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
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Could not start ffmpeg stream: {e}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Could not open ffmpeg pipe".to_string())?;
    let frame_bytes = (scale_w * scale_h * 4) as usize;
    let (tx, rx) = std::sync::mpsc::sync_channel(2);
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let cancel_flag = cancel.clone();

    std::thread::spawn(move || {
        while !cancel_flag.load(Ordering::Relaxed) {
            let mut buf = vec![0u8; frame_bytes];
            if stdout.read_exact(&mut buf).is_err() {
                break;
            }
            if tx.send((scale_w, scale_h, buf)).is_err() {
                break;
            }
        }
    });

    Ok(VideoStream {
        child,
        frame_rx: rx,
        cancel,
    })
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

/// Run ffmpeg and report `out_time=` from `-progress pipe:1` while it encodes.
pub(super) fn run_ffmpeg_watch(
    args: &[String],
    cancel: &AtomicBool,
    child_slot: &Mutex<Option<std::process::Child>>,
    mut on_time_ms: impl FnMut(i64),
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Stopped.".into());
    }
    let mut full = Vec::with_capacity(args.len() + 3);
    full.push("-nostats".into());
    full.push("-progress".into());
    full.push("pipe:1".into());
    full.extend(args.iter().cloned());
    let mut child = Command::new("ffmpeg")
        .args(full.iter().map(String::as_str))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg is not available ({e})"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    {
        let mut slot = child_slot.lock().unwrap();
        *slot = Some(child);
    }
    let err_text = std::thread::scope(|scope| {
        let logged = scope.spawn(|| {
            let mut text = String::new();
            if let Some(mut pipe) = stderr {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        });
        if let Some(pipe) = stdout {
            let mut reader = BufReader::new(pipe);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Some(ms) = a1slice_core::ffmpeg_cmd::parse_ffmpeg_time_ms(&line) {
                            on_time_ms(ms);
                        }
                    }
                }
            }
        }
        logged.join().unwrap_or_default()
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_names_include_gpu_variant() {
        let (gpu, cpu) = a1slice_core::whisper::whisper_binary_names(
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        assert!(gpu.contains("gpu"));
        assert!(!cpu.contains("gpu"));
    }

    #[test]
    fn tail_returns_ffmpeg_failed_on_empty() {
        assert_eq!(tail("   "), "ffmpeg failed");
    }

    #[test]
    fn test_por_que_reunimos_file_if_present() {
        let mov = std::path::Path::new(r"C:\Users\water\Videos\por-que-reunimos.mov");
        if mov.is_file() {
            let (ms, w, h) = probe(mov).expect("probe should succeed");
            assert_eq!((w, h), (1920, 1080));
            assert!(ms > 0);

            let (fw, fh, pixels) = grab_frame(mov, 0, 960).expect("grab_frame should succeed");
            assert_eq!((fw, fh), (960, 540));
            assert_eq!(pixels.len(), 960 * 540 * 4);

            let start = std::time::Instant::now();
            let (sfw, sfh, spixels) = grab_frame_sized(mov, 1000, 960, Some((w, h)))
                .expect("grab_frame_sized should succeed");
            let elapsed = start.elapsed();
            assert_eq!((sfw, sfh), (960, 540));
            assert_eq!(spixels.len(), 960 * 540 * 4);
            println!("grab_frame_sized elapsed: {elapsed:?}");
        }
    }
}
