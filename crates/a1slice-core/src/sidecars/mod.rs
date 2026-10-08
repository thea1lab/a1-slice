//! Files written beside a video, plus settings and the recent list.
//!
//! JSON field names match the Electron app.

mod paths;
mod read;
mod subtitles;
mod write;

pub use paths::{
    a1_dir, analysis_path, cached_preview_path, captions_path, clips_path, framing_path,
    model_path, models_dir, preview_cache_dir, recent_path, settings_path, transcript_json_path,
    transcript_text_path, video_stem,
};
pub use read::{
    inspect_video, parse_clips_cache, read_captions, read_clips, read_framing, read_transcript,
    to_stored_clip, with_clip_status,
};
pub use subtitles::{format_srt_time, generate_srt, preview_key_for, shift_subtitles};
pub use write::{
    format_transcript_text, load_settings, read_recent, remember_recent, save_settings,
    write_captions, write_clips, write_framing, write_remembered, write_transcript,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::types::{AppSettings, CropRatio, TranscriptSegment};
    use serde_json::Value;
    #[test]
    fn srt_times_and_cues() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
        assert_eq!(format_srt_time(500), "00:00:00,500");
        assert_eq!(format_srt_time(5123), "00:00:05,123");
        assert_eq!(format_srt_time(90000), "00:01:30,000");
        assert_eq!(format_srt_time(3661001), "01:01:01,001");
        assert_eq!(generate_srt(&[]), "");
        let one = generate_srt(&[TranscriptSegment {
            start_ms: 0,
            end_ms: 2000,
            text: "Hello world".into(),
        }]);
        assert_eq!(one, "1\n00:00:00,000 --> 00:00:02,000\nHello world\n");
        let two = generate_srt(&[
            TranscriptSegment {
                start_ms: 0,
                end_ms: 2000,
                text: "First".into(),
            },
            TranscriptSegment {
                start_ms: 2000,
                end_ms: 5000,
                text: "Second".into(),
            },
        ]);
        assert_eq!(
            two,
            "1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n"
        );
    }

    #[test]
    fn older_clip_file_is_kept() {
        let clips =
            parse_clips_cache(&serde_json::json!([{ "title": "Old", "startMs": 10, "endMs": 20 }]));
        assert_eq!(clips[0].title, "Old");
        assert!(clips[0].approved.is_none());
        let status = with_clip_status(&clips);
        assert!(status[0].approved);
        assert_eq!(status[0].crop.ratio, CropRatio::Original);
    }

    #[test]
    fn dropped_clip_and_crop_round_trip() {
        let clips = parse_clips_cache(&serde_json::json!([{
            "title": "Hook",
            "startMs": 0,
            "endMs": 4000,
            "approved": false,
            "crop": { "ratio": "9:16", "cx": 0.4, "cy": 0.5, "zoom": 0.2 }
        }]));
        assert_eq!(clips[0].approved, Some(false));
        assert_eq!(clips[0].crop.unwrap().ratio, CropRatio::R9x16);
    }

    #[test]
    fn recent_list_caps_at_eight_and_moves_the_latest_first() {
        let paths: Vec<String> = (0..8).map(|i| format!("/v{i}.mp4")).collect();
        let next = remember_recent(&paths, "/v3.mp4", 8);
        assert_eq!(next[0], "/v3.mp4");
        assert_eq!(next.len(), 8);
        assert!(!next[1..].contains(&"/v3.mp4".to_string()));
    }

    #[test]
    fn unknown_settings_key_survives() {
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());
        let mut settings = AppSettings::default();
        settings.user_hint = "keep the demo".into();
        fs::create_dir_all(a1_dir()).unwrap();
        fs::write(settings_path(), r#"{"extra":"stay","provider":"claude"}"#).unwrap();
        save_settings(&settings).unwrap();
        let saved: Value =
            serde_json::from_str(&fs::read_to_string(settings_path()).unwrap()).unwrap();
        assert_eq!(saved["extra"], "stay");
        assert_eq!(saved["userHint"], "keep the demo");
        if let Some(prev) = prev {
            std::env::set_var("HOME", prev);
        }
    }
}
