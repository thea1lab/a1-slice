use crate::types::{ClipCrop, CropRatio, DEFAULT_CROP};

pub const MAX_ZOOM_SCALE: f64 = 0.42;

pub fn ratio_pair(ratio: CropRatio) -> Option<(f64, f64)> {
    Some(match ratio {
        CropRatio::Original => return None,
        CropRatio::R16x9 => (16.0, 9.0),
        CropRatio::R4x3 => (4.0, 3.0),
        CropRatio::R9x16 => (9.0, 16.0),
        CropRatio::Square => (1.0, 1.0),
    })
}

pub fn crop_output_size(ratio: CropRatio) -> Option<(i32, i32)> {
    Some(match ratio {
        CropRatio::Original => return None,
        CropRatio::R16x9 => (1920, 1080),
        CropRatio::R4x3 => (1440, 1080),
        CropRatio::R9x16 => (1080, 1920),
        CropRatio::Square => (1080, 1080),
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CropPreset {
    pub ratio: CropRatio,
    pub label: &'static str,
    pub detail: &'static str,
}

pub const CROP_PRESETS: [CropPreset; 5] = [
    CropPreset {
        ratio: CropRatio::Original,
        label: "Original",
        detail: "Full frame",
    },
    CropPreset {
        ratio: CropRatio::R9x16,
        label: "Story",
        detail: "1080×1920",
    },
    CropPreset {
        ratio: CropRatio::R16x9,
        label: "YouTube",
        detail: "1920×1080",
    },
    CropPreset {
        ratio: CropRatio::Square,
        label: "Square",
        detail: "1080×1080",
    },
    CropPreset {
        ratio: CropRatio::R4x3,
        label: "Classic",
        detail: "1440×1080",
    },
];

pub fn clamp(n: f64, min: f64, max: f64) -> f64 {
    n.min(max).max(min)
}

pub fn even(n: f64) -> i32 {
    (n / 2.0).floor().max(1.0) as i32 * 2
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub fn crop_rect(crop: ClipCrop, vw: f64, vh: f64) -> Rect {
    if crop.ratio == CropRatio::Original || vw <= 0.0 || vh <= 0.0 {
        return Rect {
            x: 0.0,
            y: 0.0,
            w: vw,
            h: vh,
        };
    }
    let (rw, rh) = ratio_pair(crop.ratio).unwrap_or((1.0, 1.0));
    let mut w = vw;
    let mut h = w * (rh / rw);
    if h > vh {
        h = vh;
        w = h * (rw / rh);
    }
    let zoom = clamp(crop.zoom, 0.0, 1.0);
    let scale = 1.0 - zoom * (1.0 - MAX_ZOOM_SCALE);
    w *= scale;
    h *= scale;
    let mut x = crop.cx * vw - w / 2.0;
    let mut y = crop.cy * vh - h / 2.0;
    x = clamp(x, 0.0, (vw - w).max(0.0));
    y = clamp(y, 0.0, (vh - h).max(0.0));
    Rect { x, y, w, h }
}

pub fn snap_crop_center(crop: ClipCrop, vw: f64, vh: f64) -> ClipCrop {
    let r = crop_rect(crop, vw, vh);
    if vw <= 0.0 || vh <= 0.0 {
        return crop;
    }
    ClipCrop {
        cx: (r.x + r.w / 2.0) / vw,
        cy: (r.y + r.h / 2.0) / vh,
        ..crop
    }
}

pub fn ffmpeg_crop_filter(crop: ClipCrop, vw: f64, vh: f64) -> Option<String> {
    if crop.ratio == CropRatio::Original || vw < 2.0 || vh < 2.0 {
        return None;
    }
    let r = crop_rect(crop, vw, vh);
    let mut x = even(r.x.round());
    let mut y = even(r.y.round());
    let mut w = even(r.w.round());
    let mut h = even(r.h.round());
    if (x + w) as f64 > vw {
        w = even(vw - x as f64);
    }
    if (y + h) as f64 > vh {
        h = even(vh - y as f64);
    }
    if x < 0 {
        x = 0;
    }
    if y < 0 {
        y = 0;
    }
    if w < 2 || h < 2 {
        return None;
    }
    let (ow, oh) = crop_output_size(crop.ratio)?;
    Some(format!("crop={w}:{h}:{x}:{y},scale={ow}:{oh}"))
}

pub struct PreviousCrop<'a> {
    pub crop: ClipCrop,
    pub from_title: Option<&'a str>,
}

pub fn crop_from_previous<'a>(
    clips: &'a [crate::types::ClipSegment],
    index: usize,
    ratio: CropRatio,
) -> PreviousCrop<'a> {
    if ratio == CropRatio::Original {
        return PreviousCrop {
            crop: DEFAULT_CROP,
            from_title: None,
        };
    }
    for clip in clips[..index.min(clips.len())].iter().rev() {
        if let Some(prev) = clip.crop {
            if prev.ratio == ratio {
                return PreviousCrop {
                    crop: prev,
                    from_title: Some(clip.title.as_str()),
                };
            }
        }
    }
    PreviousCrop {
        crop: ClipCrop {
            ratio,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.0,
        },
        from_title: None,
    }
}

pub fn contain_rect(container_w: f64, container_h: f64, video_w: f64, video_h: f64) -> Rect {
    if container_w <= 0.0 || container_h <= 0.0 {
        return Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        };
    }
    if video_w <= 0.0 || video_h <= 0.0 {
        return Rect {
            x: 0.0,
            y: 0.0,
            w: container_w,
            h: container_h,
        };
    }
    let scale = (container_w / video_w).min(container_h / video_h);
    let w = video_w * scale;
    let h = video_h * scale;
    Rect {
        x: (container_w - w) / 2.0,
        y: (container_h - h) / 2.0,
        w,
        h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ClipCategory, ClipSegment, DEFAULT_CROP};

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn even_rounds_down_to_at_least_2() {
        assert_eq!(even(1080.0), 1080);
        assert_eq!(even(1081.0), 1080);
        assert_eq!(even(1.0), 2);
    }

    #[test]
    fn crop_rect_full_frame_for_original() {
        let r = crop_rect(DEFAULT_CROP, 1920.0, 1080.0);
        assert_eq!(
            r,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0
            }
        );
    }

    #[test]
    fn crop_rect_fits_story_inside_landscape() {
        let r = crop_rect(
            ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0,
            },
            1920.0,
            1080.0,
        );
        close(r.h, 1080.0);
        close(r.w, 1080.0 * (9.0 / 16.0));
        close(r.x, (1920.0 - r.w) / 2.0);
        close(r.y, 0.0);
    }

    #[test]
    fn crop_rect_stays_inside_when_panned_to_corner() {
        let r = crop_rect(
            ClipCrop {
                ratio: CropRatio::Square,
                cx: 0.0,
                cy: 0.0,
                zoom: 0.0,
            },
            1920.0,
            1080.0,
        );
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert!(r.x + r.w <= 1920.0);
        assert!(r.y + r.h <= 1080.0);
    }

    #[test]
    fn zoom_shrinks_the_window() {
        let fit = crop_rect(
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0,
            },
            1920.0,
            1080.0,
        );
        let zoomed = crop_rect(
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.5,
                zoom: 1.0,
            },
            1920.0,
            1080.0,
        );
        assert!(zoomed.w < fit.w);
        assert!(zoomed.h < fit.h);
    }

    #[test]
    fn snap_rewrites_center_to_the_clamped_window() {
        let snapped = snap_crop_center(
            ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.0,
                cy: 0.0,
                zoom: 0.0,
            },
            1.0,
            1.0,
        );
        let r = crop_rect(snapped, 1.0, 1.0);
        close(snapped.cx, r.x + r.w / 2.0);
        close(snapped.cy, r.y + r.h / 2.0);
    }

    #[test]
    fn classic_can_reach_the_top_of_a_portrait_frame() {
        let snapped = snap_crop_center(
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.0,
                zoom: 0.0,
            },
            1080.0,
            1920.0,
        );
        let r = crop_rect(snapped, 1080.0, 1920.0);
        close(r.y, 0.0);
        close(snapped.cy, r.h / (2.0 * 1920.0));
    }

    #[test]
    fn square_space_does_not_describe_a_portrait_frame() {
        let wrong = snap_crop_center(
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.0,
                zoom: 0.0,
            },
            1.0,
            1.0,
        );
        let r = crop_rect(wrong, 1080.0, 1920.0);
        assert!(r.y > 1.0);
    }

    #[test]
    fn filter_is_none_for_original() {
        assert!(ffmpeg_crop_filter(DEFAULT_CROP, 1920.0, 1080.0).is_none());
    }

    #[test]
    fn landscape_zoom_zero_covers_a_landscape_frame() {
        let rect = crop_rect(
            ClipCrop {
                ratio: CropRatio::R16x9,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0,
            },
            1920.0,
            1080.0,
        );
        close(rect.w, 1920.0);
        close(rect.h, 1080.0);
    }

    #[test]
    fn filter_emits_even_crop_for_youtube_from_vertical() {
        let filter = ffmpeg_crop_filter(
            ClipCrop {
                ratio: CropRatio::R16x9,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0,
            },
            1080.0,
            1920.0,
        )
        .unwrap();
        let nums: Vec<i32> = filter
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        assert!(filter.starts_with("crop="));
        assert!(filter.ends_with(",scale=1920:1080"));
        let (w, h, x, y) = (nums[0], nums[1], nums[2], nums[3]);
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
        assert!(x + w <= 1080);
        assert!(y + h <= 1920);
        assert!(h < 1920);
    }

    #[test]
    fn filter_emits_even_crop_for_story() {
        let filter = ffmpeg_crop_filter(
            ClipCrop {
                ratio: CropRatio::R9x16,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0,
            },
            1920.0,
            1080.0,
        )
        .unwrap();
        assert!(filter.ends_with(",scale=1080:1920"));
        let nums: Vec<i32> = filter
            .split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        let (w, h, x, y) = (nums[0], nums[1], nums[2], nums[3]);
        assert_eq!(w % 2, 0);
        assert_eq!(h % 2, 0);
        assert!(x + w <= 1920);
        assert!(y + h <= 1080);
    }

    fn clip(title: &str, crop: ClipCrop) -> ClipSegment {
        ClipSegment {
            title: title.into(),
            start_ms: 0,
            end_ms: 1,
            category: Some(ClipCategory::Standalone),
            topic: None,
            crop: Some(crop),
            approved: None,
        }
    }

    #[test]
    fn copies_the_last_matching_ratio() {
        let clips = vec![
            clip(
                "Hook",
                ClipCrop {
                    ratio: CropRatio::R9x16,
                    cx: 0.4,
                    cy: 0.5,
                    zoom: 0.2,
                },
            ),
            clip(
                "B-roll",
                ClipCrop {
                    ratio: CropRatio::R4x3,
                    cx: 0.5,
                    cy: 0.5,
                    zoom: 0.0,
                },
            ),
            clip("CTA", DEFAULT_CROP),
        ];
        let got = crop_from_previous(&clips, 2, CropRatio::R9x16);
        assert_eq!(got.from_title, Some("Hook"));
        assert_eq!(got.crop, clips[0].crop.unwrap());
    }

    #[test]
    fn starts_centered_when_no_previous_clip_used_that_ratio() {
        let clips = vec![clip("A", DEFAULT_CROP)];
        let got = crop_from_previous(&clips, 0, CropRatio::R4x3);
        assert!(got.from_title.is_none());
        assert_eq!(
            got.crop,
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.5,
                zoom: 0.0
            }
        );
    }

    #[test]
    fn letterboxes_landscape_video_in_a_square() {
        let r = contain_rect(100.0, 100.0, 1920.0, 1080.0);
        close(r.w, 100.0);
        close(r.h, 100.0 * (1080.0 / 1920.0));
        close(r.x, 0.0);
        close(r.y, (100.0 - r.h) / 2.0);
    }
}
