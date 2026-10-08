//! FFmpeg argument builders and stderr parsers. Does not spawn ffmpeg.

mod args;
mod filters;
mod fonts;

pub use args::{
    build_copy_clip_args, build_cut_clip_args, build_preview_clip_args, extract_audio_args,
    generate_srt, parse_ffmpeg_duration, parse_ffmpeg_progress, parse_ffmpeg_video_size,
    shift_subtitles, split_wav_args, VideoSize,
};
pub use filters::{
    caption_burn_filter, escape_filter_path, format_srt_time, video_filter_for_export,
};
pub use fonts::{find_caption_font, font_cell_ratio, CaptionFontMatch};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use crate::captions::DEFAULT_CAPTION_STYLE;
    use crate::types::{
        CaptionColor, CaptionFont, CaptionPosition, CaptionSize, CaptionStyle, Ms, SubtitleExport,
        TranscriptSegment,
    };
    fn cue(start_ms: Ms, end_ms: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    fn arg_after<'a>(args: &'a [String], flag: &str) -> &'a str {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        &args[index + 1]
    }

    #[test]
    fn format_srt_time_zero() {
        assert_eq!(format_srt_time(0), "00:00:00,000");
    }

    #[test]
    fn format_srt_time_milliseconds_only() {
        assert_eq!(format_srt_time(500), "00:00:00,500");
    }

    #[test]
    fn format_srt_time_seconds_and_millis() {
        assert_eq!(format_srt_time(5123), "00:00:05,123");
    }

    #[test]
    fn format_srt_time_minutes() {
        assert_eq!(format_srt_time(90000), "00:01:30,000");
    }

    #[test]
    fn format_srt_time_hours() {
        assert_eq!(format_srt_time(3661001), "01:01:01,001");
    }

    #[test]
    fn escape_filter_path_rewrites_slashes_colons_and_quotes() {
        assert_eq!(
            escape_filter_path("C:\\Windows\\Fonts\\arial.ttf"),
            "C\\:/Windows/Fonts/arial.ttf"
        );
        assert_eq!(escape_filter_path("/tmp/it's.srt"), "/tmp/it\\'s.srt");
    }

    #[test]
    fn generate_srt_empty() {
        assert_eq!(generate_srt(&[]), "");
    }

    #[test]
    fn generate_srt_one_segment() {
        let result = generate_srt(&[cue(0, 2000, "Hello world")]);
        assert_eq!(result, "1\n00:00:00,000 --> 00:00:02,000\nHello world\n");
    }

    #[test]
    fn generate_srt_multiple_segments() {
        let result = generate_srt(&[cue(0, 2000, "First"), cue(2000, 5000, "Second")]);
        assert_eq!(
            result,
            "1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n"
        );
    }

    #[test]
    fn generate_srt_trims_whitespace() {
        let result = generate_srt(&[cue(0, 1000, "  spaced  ")]);
        assert!(result.contains("spaced"));
        assert!(!result.contains("  spaced  "));
    }

    #[test]
    fn parse_ffmpeg_progress_returns_the_last_time() {
        let stderr =
            "frame=1 time=00:00:00.04 bitrate=N/A\nframe=40 time=00:00:03.20 bitrate=N/A\n";
        assert_eq!(parse_ffmpeg_progress(stderr), Some(3));
    }

    #[test]
    fn parse_ffmpeg_progress_missing() {
        assert_eq!(parse_ffmpeg_progress("frame=1 fps=30"), None);
    }

    #[test]
    fn parse_ffmpeg_duration_centiseconds() {
        assert_eq!(
            parse_ffmpeg_duration("Duration: 00:01:23.45, start: 0.000000"),
            Some(83450)
        );
    }

    #[test]
    fn parse_ffmpeg_duration_milliseconds() {
        assert_eq!(
            parse_ffmpeg_duration("  Duration: 01:00:00.500\n"),
            Some(3600500)
        );
    }

    #[test]
    fn parse_ffmpeg_duration_missing() {
        assert_eq!(parse_ffmpeg_duration("no duration here"), None);
    }

    #[test]
    fn parse_ffmpeg_video_size_from_stream_line() {
        let stderr = "Stream #0:0(und): Video: h264 (High) (avc1 / 0x31637661), yuv420p, 1920x1080 [SAR 1:1 DAR 16:9]";
        assert_eq!(
            parse_ffmpeg_video_size(stderr),
            Some(VideoSize {
                width: 1920,
                height: 1080
            })
        );
    }

    #[test]
    fn parse_ffmpeg_video_size_missing() {
        assert_eq!(parse_ffmpeg_video_size("Duration: 00:01:00.00"), None);
    }

    #[test]
    fn build_cut_clip_args_reencodes() {
        let args = build_cut_clip_args("/in.mp4", "/out.mp4", 12.5, 30.0, None);
        assert!(!args.iter().any(|arg| arg == "copy"));
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "aac"));
        assert_eq!(arg_after(&args, "-ss"), "12.5");
        assert_eq!(arg_after(&args, "-t"), "30");
        assert_eq!(
            args,
            vec![
                "-ss",
                "12.5",
                "-i",
                "/in.mp4",
                "-t",
                "30",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "18",
                "-c:a",
                "aac",
                "-movflags",
                "+faststart",
                "-avoid_negative_ts",
                "make_zero",
                "-y",
                "/out.mp4",
            ]
        );
    }

    #[test]
    fn build_cut_clip_args_adds_filter() {
        let filter = "crop=608:1080:656:0,scale=1080:1920";
        let args = build_cut_clip_args("/in.mp4", "/out.mp4", 0.0, 5.0, Some(filter));
        assert_eq!(arg_after(&args, "-vf"), filter);
    }

    #[test]
    fn caption_burn_filter_for_large_white_bottom() {
        let mut style = DEFAULT_CAPTION_STYLE;
        style.size = CaptionSize::Large;
        let burn =
            caption_burn_filter("/tmp/a.srt", "/usr/share/fonts", "DejaVu Sans", &style, 1.0);
        assert!(burn.contains("subtitles="));
        assert!(burn.contains("FontSize=28"));
        assert!(burn.contains("Outline=0.55"));
        assert!(burn.contains("PrimaryColour=&H00FFFFFF"));
        assert!(burn.contains("Alignment=2"));
        assert!(burn.contains("MarginV=90"));
        let scaled = caption_burn_filter(
            "/tmp/a.srt",
            "/usr/share/fonts",
            "DejaVu Sans",
            &style,
            1.618,
        );
        assert!(scaled.contains("FontSize=45"));
        assert!(scaled.contains("MarginV=90"));
        assert_eq!(
            video_filter_for_export(Some("crop=1:1:0:0"), SubtitleExport::Burn, Some(&burn))
                .as_deref(),
            Some(format!("crop=1:1:0:0,{burn}")).as_deref()
        );
        assert_eq!(
            video_filter_for_export(Some("crop=1:1:0:0"), SubtitleExport::Srt, Some(&burn))
                .as_deref(),
            Some("crop=1:1:0:0")
        );
        assert_eq!(
            video_filter_for_export(None, SubtitleExport::Off, Some(&burn)),
            None
        );
    }

    #[test]
    fn caption_burn_filter_writes_colour_place_and_size() {
        let style = CaptionStyle {
            color: CaptionColor::Yellow,
            custom_color: None,
            position: CaptionPosition::Top,
            size: CaptionSize::Small,
            font_size: None,
            font: CaptionFont::Serif,
        };
        let burn = caption_burn_filter("/tmp/a.srt", "/fonts", "Noto Serif", &style, 1.0);
        assert!(burn.contains("FontName=Noto Serif"));
        assert!(burn.contains("FontSize=18"));
        assert!(burn.contains("PrimaryColour=&H004AE1FF"));
        assert!(burn.contains("Alignment=8"));
        assert!(burn.contains("MarginV=36"));
    }

    #[test]
    fn find_caption_font_prefers_bundled_face() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("NotoSerif-Regular.ttf");
        std::fs::write(&file, b"").unwrap();
        let got = find_caption_font(CaptionFont::Serif, &[dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(got.dir, dir.path().to_str().unwrap());
        assert_eq!(got.name, "Noto Serif");
        assert_eq!(got.file, file.to_str().unwrap());
        assert_eq!(got.cell_ratio, 1.0);
    }

    #[test]
    fn find_caption_font_prefers_medium_face() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("NotoSansMono-Regular.ttf"), b"").unwrap();
        let medium = dir.path().join("NotoSansMono-Medium.ttf");
        std::fs::write(&medium, b"").unwrap();
        let got = find_caption_font(CaptionFont::Mono, &[dir.path().to_str().unwrap()]).unwrap();
        assert_eq!(got.name, "Noto Sans Mono Medium");
        assert_eq!(got.file, medium.to_str().unwrap());
    }

    #[test]
    fn font_cell_ratio_of_noto_when_installed() {
        let mono = "/usr/share/fonts/noto/NotoSansMono-Medium.ttf";
        let sans = "/usr/share/fonts/noto/NotoSans-Medium.ttf";
        if Path::new(mono).exists() {
            assert!((font_cell_ratio(mono) - 1.618).abs() < 5e-4);
        }
        if Path::new(sans).exists() {
            assert!((font_cell_ratio(sans) - 1.519).abs() < 5e-4);
        }
    }

    #[test]
    fn font_cell_ratio_reads_dejavu_when_installed() {
        let path = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";
        if Path::new(path).exists() {
            assert!((font_cell_ratio(path) - 1.1640625).abs() < 1e-9);
        }
    }

    #[test]
    fn font_cell_ratio_missing_file_is_one() {
        assert_eq!(font_cell_ratio("/no/such/font.ttf"), 1.0);
    }

    #[test]
    fn build_preview_clip_args_reencodes_yuv420p() {
        let args = build_preview_clip_args("/in.mp4", "/preview.mp4", 5.0, 8.0);
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "veryfast"));
        assert!(args.iter().any(|arg| arg == "yuv420p"));
        assert_eq!(
            arg_after(&args, "-vf"),
            "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p"
        );
        assert!(args.iter().any(|arg| arg.contains("scale=")));
        assert!(!args.iter().any(|arg| arg == "copy"));
        assert_eq!(arg_after(&args, "-ss"), "5");
        assert_eq!(arg_after(&args, "-t"), "8");
    }

    #[test]
    fn build_copy_clip_args_stream_copies() {
        assert_eq!(
            build_copy_clip_args("/in.mp4", "/out.mp4", 1.25, 4.0),
            vec![
                "-ss",
                "1.25",
                "-i",
                "/in.mp4",
                "-t",
                "4",
                "-c",
                "copy",
                "-avoid_negative_ts",
                "make_zero",
                "-y",
                "/out.mp4",
            ]
        );
    }

    #[test]
    fn extract_audio_and_split_wav_args() {
        assert_eq!(
            extract_audio_args("/in.mp4", "/tmp/out.wav"),
            vec![
                "-i",
                "/in.mp4",
                "-af",
                "afftdn=nr=12:nf=-20:tn=1",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-f",
                "wav",
                "-y",
                "/tmp/out.wav",
            ]
        );
        assert_eq!(
            split_wav_args("/in.wav", "/in-chunk0.wav", 0.0, 300.0),
            vec![
                "-ss",
                "0",
                "-i",
                "/in.wav",
                "-t",
                "300",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-y",
                "/in-chunk0.wav",
            ]
        );
    }

    #[test]
    fn shift_subtitles_clips_to_the_window() {
        let cues = vec![
            cue(0, 500, "before"),
            cue(500, 1500, "overlap"),
            cue(2000, 2500, "inside"),
            cue(3000, 4000, "after"),
        ];
        let shifted = shift_subtitles(&cues, 1000, 3000);
        assert_eq!(
            shifted,
            vec![cue(0, 500, "overlap"), cue(1000, 1500, "inside")]
        );
    }
}
