//! Cut, reframe, and burn-caption exports.

use a1slice_core::clip_find::format_clip_transcript;
use a1slice_core::crop::ffmpeg_crop_filter;
use a1slice_core::ffmpeg_cmd::{caption_burn_filter, find_caption_font};
use a1slice_core::sidecars::{self, generate_srt, shift_subtitles};
use a1slice_core::types::{
    CaptionLook, CaptionStyle, ClipCrop, ClipSegment, CropRatio, PipelineStage, ProgressUpdate,
    TranscriptSegment,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::process::{probe, run_ffmpeg, run_ffmpeg_watch};

/// Percent of an encode, held under 100 until the file is actually written.
pub(crate) fn encode_percent(time_ms: i64, duration_ms: i64) -> f64 {
    if duration_ms <= 0 {
        return 0.0;
    }
    ((time_ms.max(0) as f64 / duration_ms as f64) * 100.0).clamp(0.0, 99.0)
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
    mut on_progress: impl FnMut(ProgressUpdate),
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
    on_progress(ProgressUpdate {
        stage: PipelineStage::Cutting,
        message: "Reframing the picture…".into(),
        percent: 0.0,
    });
    let last = std::cell::Cell::new(0.0_f64);
    run_ffmpeg_watch(
        &cut_args(video, &file, 0.0, duration_sec, filter.as_deref()),
        cancel,
        child_slot,
        |time_ms| {
            let percent = encode_percent(time_ms, duration);
            if percent - last.get() >= 1.0 {
                last.set(percent);
                on_progress(ProgressUpdate {
                    stage: PipelineStage::Cutting,
                    message: "Reframing the picture…".into(),
                    percent,
                });
            }
        },
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
    mut on_progress: impl FnMut(ProgressUpdate),
) -> Result<PathBuf, String> {
    let (dir, stem) = sidecars::video_stem(video);
    if look == CaptionLook::Srt {
        let path = dir.join(format!("{stem}.srt"));
        std::fs::write(&path, generate_srt(cues)).map_err(|e| e.to_string())?;
        on_progress(ProgressUpdate {
            stage: PipelineStage::Cutting,
            message: "Writing the caption file…".into(),
            percent: 100.0,
        });
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
    let font = find_caption_font(style.font, &[])
        .ok_or_else(|| "No caption font found on this computer".to_string())?;
    let filter = caption_burn_filter(
        &srt.display().to_string(),
        &font.dir,
        &font.name,
        style,
        font.cell_ratio,
    );
    let duration_sec = (duration as f64 / 1000.0).max(0.2);
    on_progress(ProgressUpdate {
        stage: PipelineStage::Cutting,
        message: "Burning the words into the picture…".into(),
        percent: 0.0,
    });
    let last = std::cell::Cell::new(0.0_f64);
    let result = run_ffmpeg_watch(
        &cut_args(video, &file, 0.0, duration_sec, Some(&filter)),
        cancel,
        child_slot,
        |time_ms| {
            let percent = encode_percent(time_ms, duration);
            if percent - last.get() >= 1.0 {
                last.set(percent);
                on_progress(ProgressUpdate {
                    stage: PipelineStage::Cutting,
                    message: "Burning the words into the picture…".into(),
                    percent,
                });
            }
        },
    );
    let _ = std::fs::remove_file(&srt);
    result?;
    Ok(out_dir)
}

#[cfg(test)]
mod tests {
    use super::encode_percent;

    #[test]
    fn encode_percent_tracks_the_file_and_holds_the_last_step() {
        assert_eq!(encode_percent(0, 10_000), 0.0);
        assert_eq!(encode_percent(5_000, 10_000), 50.0);
        assert_eq!(encode_percent(20_000, 10_000), 99.0);
        assert_eq!(encode_percent(-10, 10_000), 0.0);
        assert_eq!(encode_percent(100, 0), 0.0);
    }
}
