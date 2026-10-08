//! Cut, reframe, and burn-caption exports.

use a1slice_core::clip_find::format_clip_transcript;
use a1slice_core::crop::ffmpeg_crop_filter;
use a1slice_core::sidecars::{self, generate_srt, shift_subtitles};
use a1slice_core::types::{
    CaptionFont, CaptionLook, CaptionStyle, ClipCrop, ClipSegment, CropRatio, PipelineStage,
    ProgressUpdate, TranscriptSegment,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::process::{probe, run_ffmpeg};

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
