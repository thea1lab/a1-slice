//! Runs ffmpeg, whisper-cli, and the installed agents.
//!
//! The window sends work here and reads progress back on a channel.

mod agent_run;
mod clip_parse;
mod export_job;
mod job;
mod process;
mod spawn;
mod transcribe;

pub use agent_run::{find_clips, fix_words};
pub use export_job::{export_captions, export_kept_clips, export_reframe};
pub use job::{list_agents, JobDone, Running};
pub use process::{grab_frame, probe, which};
pub use spawn::{cancel_job, open_path, spawn_job};
pub use transcribe::transcribe_video;

#[cfg(test)]
mod tests {
    use super::*;
    use a1slice_core::types::{CaptionFont, CaptionLook, CropRatio, TranscriptSegment};
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
