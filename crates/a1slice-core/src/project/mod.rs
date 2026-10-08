//! Transcript, clip, framing, and caption files beside a video.
//!
//! Parsers match `src/shared/project.ts`. Reads and writes match `projectStore.ts`.

mod parse;
mod paths;
mod store;

pub use parse::{
    format_transcript_text, parse_caption_document, parse_captions, parse_clips_cache, parse_crop,
    parse_framing, parse_srt, parse_transcript_lines, preview_cache_key, remember_recent,
    to_stored_clip, with_clip_status,
};
pub use paths::{
    a1slice_dir, analysis_path, captions_path, clips_path, framing_path, transcript_json_path,
    transcript_text_path, video_stem, VideoStem,
};
pub use store::{
    cached_preview_path, cached_preview_path_in, inspect_video, is_cached_preview,
    is_cached_preview_in, preview_cache_dir, preview_cache_dir_in, read_captions, read_clips,
    read_framing, read_project_files, read_recent, read_recent_in, read_transcript, recent_path,
    recent_path_in, write_captions, write_clips, write_framing, write_remembered,
    write_remembered_in, write_transcript, ClipRead, ProjectFiles,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use crate::types::{
        CaptionColor, CaptionFont, CaptionLook, CaptionPosition, CaptionProject, CaptionSize,
        CaptionSource, CaptionStyle, ClipCategory, ClipCrop, ClipSegment, CropRatio,
        TranscriptSegment, DEFAULT_CROP,
    };
    use serde_json::json;
    fn segments() -> Vec<TranscriptSegment> {
        vec![
            TranscriptSegment {
                start_ms: 0,
                end_ms: 2000,
                text: "Hello world".into(),
            },
            TranscriptSegment {
                start_ms: 2000,
                end_ms: 5000,
                text: "Second".into(),
            },
        ]
    }

    fn style(
        color: CaptionColor,
        position: CaptionPosition,
        size: CaptionSize,
        font: CaptionFont,
    ) -> CaptionStyle {
        CaptionStyle {
            color,
            custom_color: None,
            position,
            size,
            font_size: None,
            font,
        }
    }

    #[test]
    fn keeps_crop_and_a_dropped_clip() {
        let clips = parse_clips_cache(&json!([{
            "title": "Hook",
            "startMs": 0,
            "endMs": 4000,
            "approved": false,
            "crop": { "ratio": "9:16", "cx": 0.4, "cy": 0.5, "zoom": 0.2 }
        }]));
        assert_eq!(clips[0].approved, Some(false));
        assert_eq!(
            clips[0].crop,
            Some(ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.4,
                cy: 0.5,
                zoom: 0.2
            })
        );
    }

    #[test]
    fn loads_an_older_file_that_only_has_title_and_times() {
        let clips = parse_clips_cache(&json!([{ "title": "Old", "startMs": 10, "endMs": 20 }]));
        assert_eq!(
            clips,
            vec![ClipSegment {
                title: "Old".into(),
                start_ms: 10,
                end_ms: 20,
                category: None,
                topic: None,
                crop: None,
                approved: None,
            }]
        );
        let with_status = with_clip_status(&clips);
        assert_eq!(with_status[0].id, "0");
        assert!(with_status[0].approved);
        assert_eq!(with_status[0].crop, DEFAULT_CROP);
    }

    #[test]
    fn drops_the_ui_id_and_records_approval() {
        let stored = to_stored_clip(&ClipSegment {
            title: "A".into(),
            start_ms: 1,
            end_ms: 2,
            category: None,
            topic: None,
            crop: None,
            approved: Some(true),
        });
        assert_eq!(
            stored,
            ClipSegment {
                title: "A".into(),
                start_ms: 1,
                end_ms: 2,
                category: None,
                topic: None,
                crop: None,
                approved: Some(true),
            }
        );
    }

    #[test]
    fn returns_the_saved_story_frame() {
        let framing = parse_framing(&json!({
            "9:16": { "ratio": "9:16", "cx": 0.42, "cy": 0.5, "zoom": 0.1 },
            "nope": { "ratio": "9:16", "cx": 0.1, "cy": 0.1, "zoom": 0 }
        }));
        assert_eq!(
            framing.get(&CropRatio::R9x16).copied(),
            Some(ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.42,
                cy: 0.5,
                zoom: 0.1
            })
        );
        assert!(framing.get(&CropRatio::R16x9).is_none());
    }

    #[test]
    fn parses_an_srt_back_into_segments() {
        let srt = [
            "1",
            "00:00:00,000 --> 00:00:02,000",
            "Hello world",
            "",
            "2",
            "00:00:02,000 --> 00:00:05,000",
            "Second",
            "",
        ]
        .join("\n");
        assert_eq!(parse_srt(&srt), segments());
    }

    #[test]
    fn reads_a_saved_caption_project() {
        assert_eq!(
            parse_captions(&json!({
                "source": "manual",
                "look": "burn",
                "style": { "color": "yellow", "position": "top", "size": "medium", "font": "serif" },
                "cues": segments()
            })),
            Some(CaptionProject {
                source: CaptionSource::Manual,
                look: CaptionLook::Burn,
                style: style(
                    CaptionColor::Yellow,
                    CaptionPosition::Top,
                    CaptionSize::Medium,
                    CaptionFont::Serif
                ),
                cues: segments(),
                file_path: None,
            })
        );
    }

    #[test]
    fn reads_a_chosen_caption_file_only_for_pasted_words() {
        assert_eq!(
            parse_captions(&json!({
                "source": "manual",
                "look": "burn",
                "cues": segments(),
                "filePath": "/videos/other.srt"
            }))
            .and_then(|project| project.file_path),
            Some("/videos/other.srt".into())
        );
        assert_eq!(
            parse_captions(&json!({
                "source": "transcript",
                "look": "srt",
                "cues": [],
                "filePath": "/videos/other.srt"
            }))
            .and_then(|project| project.file_path),
            None
        );
    }

    #[test]
    fn reads_a_transcript_file_and_prefers_a_subtitle_file() {
        assert_eq!(
            parse_transcript_lines("0:00  Hello world\n0:02  Second\n"),
            vec![
                TranscriptSegment {
                    start_ms: 0,
                    end_ms: 2000,
                    text: "Hello world".into()
                },
                TranscriptSegment {
                    start_ms: 2000,
                    end_ms: 4000,
                    text: "Second".into()
                },
            ]
        );
        assert_eq!(
            parse_transcript_lines("1:02:03  Hello")
                .into_iter()
                .map(|line| line.start_ms)
                .collect::<Vec<_>>(),
            vec![3_723_000]
        );
        let srt = ["1", "00:00:01,000 --> 00:00:03,000", "Hello", ""].join("\n");
        assert_eq!(
            parse_caption_document(&srt),
            vec![TranscriptSegment {
                start_ms: 1000,
                end_ms: 3000,
                text: "Hello".into()
            }]
        );
        assert_eq!(
            parse_caption_document("0:12  Hello there"),
            vec![TranscriptSegment {
                start_ms: 12000,
                end_ms: 14000,
                text: "Hello there".into()
            }]
        );
    }

    #[test]
    fn keeps_an_older_burn_preset_as_a_size() {
        assert_eq!(
            parse_captions(
                &json!({ "source": "manual", "look": "burn-large", "cues": segments() })
            )
            .unwrap()
            .style
            .size,
            CaptionSize::Large
        );
        assert_eq!(
            parse_captions(&json!({ "source": "manual", "look": "burn-small", "cues": [] }))
                .unwrap()
                .style,
            style(
                CaptionColor::White,
                CaptionPosition::Bottom,
                CaptionSize::Small,
                CaptionFont::Sans
            )
        );
        assert_eq!(
            parse_captions(&json!({ "source": "transcript", "look": "srt", "cues": [] }))
                .unwrap()
                .look,
            CaptionLook::Srt
        );
    }

    #[test]
    fn writes_a_readable_transcript() {
        let text = format_transcript_text(&segments());
        assert!(text.contains("0:00  Hello world"));
        assert!(text.contains("0:02  Second"));
        assert!(format_transcript_text(&[TranscriptSegment {
            start_ms: 3_723_000,
            end_ms: 3_724_000,
            text: "Hello".into(),
        }])
        .contains("1:02:03  Hello"));
    }

    #[test]
    fn names_the_readable_transcript_beside_the_video() {
        assert_eq!(
            transcript_text_path("/videos/talk.mp4"),
            "/videos/talk-transcript.txt"
        );
        assert_eq!(
            transcript_text_path(r"C:\videos\talk.mov"),
            r"C:\videos\talk-transcript.txt"
        );
        assert_eq!(
            transcript_text_path("/videos/my.clip.mkv"),
            "/videos/my.clip-transcript.txt"
        );
        assert_eq!(transcript_text_path("talk.webm"), "talk-transcript.txt");
    }

    #[test]
    fn puts_the_newest_path_first_and_caps_the_list() {
        let paths = ["a", "b", "c", "d", "e", "f", "g", "h"];
        assert_eq!(
            remember_recent(&paths, "c", 8),
            vec!["c", "a", "b", "d", "e", "f", "g", "h"]
        );
        assert_eq!(remember_recent(&paths, "new", 3), vec!["new", "a", "b"]);
    }

    #[test]
    fn preview_cache_key_stays_stable_for_the_same_file_and_range() {
        let key = preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0);
        assert_eq!(key, "ea259571");
        assert_eq!(preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0), key);
        assert_eq!(preview_cache_key("/v.mp4", 10, 20.4, 0.0, 5000.0), key);
    }

    #[test]
    fn preview_cache_key_changes_when_the_range_or_the_file_changes() {
        let key = preview_cache_key("/v.mp4", 10, 20.0, 0.0, 5000.0);
        assert_ne!(preview_cache_key("/v.mp4", 10, 20.0, 1000.0, 5000.0), key);
        assert_ne!(preview_cache_key("/v.mp4", 10, 21.0, 0.0, 5000.0), key);
    }

    #[test]
    fn writes_sidecar_files_and_reads_them_back() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("talk.mp4");
        fs::write(&video, b"video").unwrap();
        let text_path = write_transcript(&video, &segments()).unwrap();
        assert_eq!(text_path, dir.path().join("talk-transcript.txt"));
        let readable = fs::read_to_string(&text_path).unwrap();
        assert!(readable.ends_with('\n'));
        assert!(readable.contains("0:00  Hello world"));
        assert_eq!(read_transcript(&video), segments());

        let clip = ClipSegment {
            title: "Hook".into(),
            start_ms: 0,
            end_ms: 4000,
            category: Some(ClipCategory::Standalone),
            topic: Some("open".into()),
            crop: None,
            approved: Some(false),
        };
        write_clips(&video, &[clip.clone()], Some("raw answer")).unwrap();
        let loaded = read_clips(&video);
        assert_eq!(loaded.raw_response, "raw answer");
        assert_eq!(loaded.clips[0].approved, Some(false));
        assert_eq!(loaded.clips[0].topic.as_deref(), Some("open"));

        let mut framing = BTreeMap::new();
        framing.insert(
            CropRatio::R9x16,
            ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.4,
                cy: 0.5,
                zoom: 0.2,
            },
        );
        write_framing(&video, &framing).unwrap();
        assert_eq!(read_framing(&video), framing);

        let inspection = inspect_video(&video);
        assert!(inspection.has_transcript);
        assert_eq!(inspection.segment_count, 2);
        assert!(inspection.has_clips);
        assert!(inspection.has_framing);
        assert!(!inspection.has_captions);
        assert!(!inspect_video(Path::new("/missing/nope.mp4")).has_transcript);
    }

    #[test]
    fn remembers_paths_inside_the_given_directory() {
        let dir = tempfile::tempdir().unwrap();
        let a1 = dir.path().join("a1");
        let first = write_remembered_in(&a1, "/v/a.mp4").unwrap();
        assert_eq!(first, vec!["/v/a.mp4".to_string()]);
        let second = write_remembered_in(&a1, "/v/b.mp4").unwrap();
        assert_eq!(second, vec!["/v/b.mp4".to_string(), "/v/a.mp4".to_string()]);
        assert_eq!(read_recent_in(&a1), second);
        let again = write_remembered_in(&a1, "/v/a.mp4").unwrap();
        assert_eq!(again[0], "/v/a.mp4");
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn cached_preview_paths_stay_inside_the_cache_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = preview_cache_dir_in(dir.path());
        let file = cached_preview_path_in(dir.path(), "ea259571");
        assert!(is_cached_preview_in(dir.path(), &file));
        assert!(is_cached_preview_in(dir.path(), &root));
        assert!(!is_cached_preview_in(
            dir.path(),
            dir.path().join("previews-evil").join("x.mp4")
        ));
    }
}
