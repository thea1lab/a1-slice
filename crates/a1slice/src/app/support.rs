//! Clocks, caption style helpers, recent videos, and playback audio.

use crate::backend::{self};
use a1slice_core::sidecars::{self};
use a1slice_core::types::*;
use a1slice_core::wizard::WizardState;

pub(super) fn clock(ms: i64) -> String {
    a1slice_core::preview::format_playback_clock(ms.max(0) as f64)
}

pub(super) fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

pub(super) fn tool_bar_title(screen: Screen) -> &'static str {
    match screen {
        Screen::Home => "A1 Slice",
        Screen::Transcribe | Screen::TranscribeDone => "Transcribe",
        Screen::Find | Screen::Review => "Find best parts",
        Screen::Export => "Export",
        Screen::Reframe => "Reframe",
        Screen::Captions | Screen::FixWords => "Captions",
    }
}

pub(super) fn settings_of(state: &WizardState) -> AppSettings {
    AppSettings {
        provider: state.provider,
        model: state.model.clone(),
        api_key: state.api_key.clone(),
        api_keys: state.api_keys.clone(),
        user_hint: state.user_hint.clone(),
        language: state.language,
        entropy_thold: state.entropy_thold,
        max_context: state.max_context,
        beam_size: state.beam_size,
        temperature_inc: state.temperature_inc,
    }
}

pub(super) fn default_style() -> CaptionStyle {
    CaptionStyle {
        color: CaptionColor::White,
        custom_color: None,
        position: CaptionPosition::Bottom,
        size: CaptionSize::Large,
        font_size: None,
        font: CaptionFont::Sans,
    }
}

pub(super) fn current_crop(state: &WizardState, ratio: CropRatio) -> ClipCrop {
    if ratio == CropRatio::Original {
        DEFAULT_CROP
    } else {
        state.framing.get(&ratio).copied().unwrap_or(ClipCrop {
            ratio,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        })
    }
}

/// The line drawn on the picture. The cue under the playhead wins.
/// Before the first cue starts, the first line is shown so the style is visible.
pub(super) fn caption_on_picture<'a>(
    cues: &'a [TranscriptSegment],
    playhead_ms: i64,
) -> Option<&'a str> {
    let spoken = |text: &'a str| (!text.trim().is_empty()).then_some(text);
    cues.iter()
        .find(|cue| playhead_ms >= cue.start_ms && playhead_ms <= cue.end_ms)
        .and_then(|cue| spoken(&cue.text))
        .or_else(|| cues.first().and_then(|cue| spoken(&cue.text)))
}

pub(super) fn active_cues(state: &WizardState, source: CaptionSource) -> Vec<TranscriptSegment> {
    if source == CaptionSource::Manual {
        if let Some(captions) = &state.captions {
            if captions.source == CaptionSource::Manual && !captions.cues.is_empty() {
                return captions.cues.clone();
            }
        }
    }
    state.segments.clone()
}

pub(super) fn caption_project(
    source: CaptionSource,
    look: CaptionLook,
    style: CaptionStyle,
    cues: Vec<TranscriptSegment>,
    file_path: Option<String>,
) -> CaptionProject {
    CaptionProject {
        source,
        look,
        style,
        cues,
        file_path,
    }
}

pub(super) fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", hex.to_ascii_lowercase()))
    } else {
        None
    }
}

pub(super) fn load_recent() -> Vec<(String, String)> {
    sidecars::read_recent()
        .into_iter()
        .map(|path| {
            let info = sidecars::inspect_video(std::path::Path::new(&path));
            let mut parts = Vec::new();
            if info.has_transcript {
                parts.push("Transcript saved".to_string());
            }
            if info.has_clips {
                parts.push(if info.clip_count == 1 {
                    "1 clip".into()
                } else {
                    format!("{} clips", info.clip_count)
                });
            }
            if info.has_framing {
                parts.push("Frame saved".into());
            }
            if info.has_captions {
                parts.push("Captions saved".into());
            }
            (path, parts.join(" · "))
        })
        .collect()
}

fn resolve_player_binary(name: &str) -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        if name == "ffplay" {
            let choco_real = std::path::PathBuf::from(
                r"C:\ProgramData\chocolatey\lib\ffmpeg\tools\ffmpeg\bin\ffplay.exe",
            );
            if choco_real.is_file() {
                return Some(choco_real);
            }
        }
    }
    let found = backend::which(name)?;
    #[cfg(windows)]
    {
        let path_str = found.to_string_lossy();
        if path_str.contains("chocolatey\\bin") {
            let candidate = found
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join(format!("lib/ffmpeg/tools/ffmpeg/bin/{name}.exe")));
            if let Some(target) = candidate {
                if target.is_file() {
                    return Some(target);
                }
            }
        }
    }
    Some(found)
}

pub(super) fn kill_player_processes() {
    #[cfg(windows)]
    {
        let mut kill_cmd = std::process::Command::new("taskkill");
        kill_cmd
            .args(["/F", "/IM", "ffplay.exe", "/IM", "mpv.exe"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        {
            use std::os::windows::process::CommandExt;
            kill_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let _ = kill_cmd.status();
    }
}

/// End a playback process and wait until it is gone.
///
/// On Windows, shims or wrappers can spawn the actual player as a child
/// process. Calling taskkill /F /T terminates the entire process tree so
/// no audio keeps playing in the background.
pub(super) fn stop_child(mut child: std::process::Child) {
    #[cfg(windows)]
    {
        let pid = child.id();
        let mut kill_cmd = std::process::Command::new("taskkill");
        kill_cmd
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        {
            use std::os::windows::process::CommandExt;
            kill_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let _ = kill_cmd.status();
    }
    let _ = child.kill();
    let _ = child.wait();
    kill_player_processes();
}

pub(super) fn start_audio(path: &str, at_ms: i64, volume: f32) -> Option<std::process::Child> {
    kill_player_processes();
    let sec = format!("{:.3}", at_ms.max(0) as f64 / 1000.0);
    let vol = ((volume.clamp(0.0, 1.0)) * 100.0) as i32;
    if let Some(player) = resolve_player_binary("ffplay") {
        let mut cmd = std::process::Command::new(player);
        cmd.args([
            "-nodisp",
            "-autoexit",
            "-ss",
            &sec,
            "-volume",
            &vol.to_string(),
            "-loglevel",
            "quiet",
            path,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        return cmd.spawn().ok();
    }
    if let Some(player) = resolve_player_binary("mpv") {
        let mut cmd = std::process::Command::new(player);
        cmd.args([
            "--no-video",
            "--really-quiet",
            &format!("--start={sec}"),
            &format!("--volume={vol}"),
            path,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        return cmd.spawn().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{caption_on_picture, file_name, normalize_hex, tool_bar_title};
    use a1slice_core::types::{Screen, TranscriptSegment};

    fn cue(start_ms: i64, end_ms: i64, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    #[test]
    fn hex_colour_gains_a_hash_and_lowercase() {
        assert_eq!(normalize_hex("  #FFb83E "), Some("#ffb83e".into()));
        assert_eq!(normalize_hex("ffb83e"), Some("#ffb83e".into()));
        assert_eq!(normalize_hex("#fff"), None);
        assert_eq!(normalize_hex("not-a-colour"), None);
    }

    #[test]
    fn file_name_keeps_the_last_path_piece() {
        assert_eq!(file_name("/tmp/interview.mp4"), "interview.mp4");
        assert_eq!(file_name("interview.mp4"), "interview.mp4");
    }

    #[test]
    fn tool_bar_names_the_tool_on_every_screen() {
        assert_eq!(tool_bar_title(Screen::Transcribe), "Transcribe");
        assert_eq!(tool_bar_title(Screen::TranscribeDone), "Transcribe");
        assert_eq!(tool_bar_title(Screen::Find), "Find best parts");
        assert_eq!(tool_bar_title(Screen::Review), "Find best parts");
        assert_eq!(tool_bar_title(Screen::Export), "Export");
        assert_eq!(tool_bar_title(Screen::Reframe), "Reframe");
        assert_eq!(tool_bar_title(Screen::Captions), "Captions");
        assert_eq!(tool_bar_title(Screen::FixWords), "Captions");
    }

    #[test]
    fn stop_child_ends_the_process() {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sleep");
        let pid = child.id();
        super::stop_child(child);
        assert!(
            std::fs::metadata(format!("/proc/{pid}")).is_err(),
            "process {pid} was still running"
        );
    }

    #[test]
    fn caption_on_picture_shows_the_line_under_the_playhead() {
        let cues = vec![cue(0, 1000, "Hello"), cue(1000, 2000, "There")];
        assert_eq!(caption_on_picture(&cues, 1500), Some("There"));
        assert_eq!(caption_on_picture(&cues, 250), Some("Hello"));
    }

    #[test]
    fn caption_on_picture_uses_the_first_line_before_it_starts() {
        let cues = vec![cue(1500, 3000, "Later")];
        assert_eq!(caption_on_picture(&cues, 0), Some("Later"));
        assert_eq!(caption_on_picture(&[], 0), None);
        assert_eq!(caption_on_picture(&[cue(0, 10, "  ")], 5), None);
    }

    #[test]
    fn start_and_stop_audio_leaves_no_lingering_process() {
        let mov = std::path::Path::new(r"C:\Users\water\Videos\por-que-reunimos.mov");
        if mov.is_file() {
            let child = super::start_audio(mov.to_str().unwrap(), 0, 0.0);
            assert!(child.is_some(), "start_audio returned None!");
            let child = child.unwrap();
            let pid = child.id();
            println!("Child PID: {pid}");
            super::stop_child(child);
            std::thread::sleep(std::time::Duration::from_millis(200));
            let proc_check = std::process::Command::new("tasklist")
                .args(["/FI", "IMAGENAME eq ffplay.exe", "/NH"])
                .output()
                .unwrap();
            let text = String::from_utf8_lossy(&proc_check.stdout);
            println!("Tasklist output: {text}");
            assert!(
                !text.contains("ffplay.exe"),
                "ffplay was still running: {text}"
            );
        }
    }
}
