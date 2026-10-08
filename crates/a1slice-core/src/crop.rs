use crate::types::{ClipCrop, CropRatio, DEFAULT_CROP};

pub const MAX_ZOOM_SCALE: f64 = 0.42;
/// Slider and wheel range. 0 is the fitted frame, 1 is the old maximum, 2 is twice as tight.
pub const MAX_CROP_ZOOM: f64 = 2.0;

/// Window size relative to the fitted frame. 1 keeps today's zoom of 1, and 2 halves that window.
pub fn zoom_window_scale(zoom: f64) -> f64 {
    let zoom = clamp(zoom, 0.0, MAX_CROP_ZOOM);
    if zoom <= 1.0 {
        1.0 - zoom * (1.0 - MAX_ZOOM_SCALE)
    } else {
        MAX_ZOOM_SCALE / zoom
    }
}

/// Inverse of `zoom_window_scale`. `scale` is 1 at the fitted frame and smaller when tighter.
fn zoom_for_window_scale(scale: f64) -> f64 {
    let tightest = zoom_window_scale(MAX_CROP_ZOOM);
    let scale = clamp(scale, tightest, 1.0);
    if scale >= MAX_ZOOM_SCALE {
        (1.0 - scale) / (1.0 - MAX_ZOOM_SCALE)
    } else {
        MAX_ZOOM_SCALE / scale
    }
}

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
    let scale = zoom_window_scale(crop.zoom);
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

/// Move the frame with the pointer. `dx_px` and `dy_px` are in the same space as `vw` and `vh`.
///
/// A positive `dx_px` moves the frame right. One wheel notch is about 40 points and
/// changes zoom by 0.06, matching the main player: content moving down zooms in.
pub fn pan_crop(crop: ClipCrop, dx_px: f64, dy_px: f64, vw: f64, vh: f64) -> ClipCrop {
    if vw <= 0.0 || vh <= 0.0 {
        return crop;
    }
    snap_crop_center(
        ClipCrop {
            cx: crop.cx + dx_px / vw,
            cy: crop.cy + dy_px / vh,
            ..crop
        },
        vw,
        vh,
    )
}

const SCROLL_POINTS_PER_NOTCH: f64 = 40.0;
const SCROLL_ZOOM_STEP: f64 = 0.06;

pub fn zoom_from_scroll(zoom: f64, content_delta_y: f64) -> f64 {
    clamp(
        zoom + content_delta_y * (SCROLL_ZOOM_STEP / SCROLL_POINTS_PER_NOTCH),
        0.0,
        MAX_CROP_ZOOM,
    )
}

/// Zoom from a corner handle. The pointer is in the same space as `vw` and `vh`.
///
/// Distance is measured from the current frame, so a click on its corner keeps
/// the zoom. Pulling toward the middle zooms in. The widest frame still fits
/// the picture, and the tightest reach is 200%.
pub fn zoom_from_corner(
    crop: ClipCrop,
    pointer_x: f64,
    pointer_y: f64,
    vw: f64,
    vh: f64,
) -> ClipCrop {
    if vw <= 0.0 || vh <= 0.0 {
        return crop;
    }
    let fitted = crop_rect(ClipCrop { zoom: 0.0, ..crop }, vw, vh);
    let max_dist = ((fitted.w / 2.0).powi(2) + (fitted.h / 2.0).powi(2))
        .sqrt()
        .max(1e-6);
    let frame = crop_rect(crop, vw, vh);
    let cx = frame.x + frame.w / 2.0;
    let cy = frame.y + frame.h / 2.0;
    let dist = ((pointer_x - cx).powi(2) + (pointer_y - cy).powi(2)).sqrt();
    snap_crop_center(
        ClipCrop {
            zoom: zoom_for_window_scale(dist / max_dist),
            ..crop
        },
        vw,
        vh,
    )
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
        let tighter = crop_rect(
            ClipCrop {
                ratio: CropRatio::R4x3,
                cx: 0.5,
                cy: 0.5,
                zoom: MAX_CROP_ZOOM,
            },
            1920.0,
            1080.0,
        );
        assert!(
            (tighter.w - zoomed.w * 0.5).abs() < 0.5,
            "200% should be half the 100% window, got {} vs {}",
            tighter.w,
            zoomed.w
        );
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

    fn square_crop() -> ClipCrop {
        ClipCrop {
            ratio: CropRatio::Square,
            cx: 0.5,
            cy: 0.5,
            zoom: 0.35,
        }
    }

    #[test]
    fn pan_moves_the_frame_with_the_pointer() {
        let crop = square_crop();
        let before = crop_rect(crop, 1920.0, 1080.0);
        let moved = pan_crop(crop, 80.0, 40.0, 1920.0, 1080.0);
        let after = crop_rect(moved, 1920.0, 1080.0);
        assert!(
            after.x > before.x + 40.0,
            "frame should follow a right drag"
        );
        assert!(after.y > before.y + 10.0, "frame should follow a down drag");
    }

    #[test]
    fn wheel_up_zooms_in_and_wheel_down_zooms_out() {
        let closer = zoom_from_scroll(0.2, 40.0);
        close(closer, 0.26);
        let wider = zoom_from_scroll(closer, -40.0);
        close(wider, 0.2);
        assert_eq!(zoom_from_scroll(MAX_CROP_ZOOM, 80.0), MAX_CROP_ZOOM);
        assert_eq!(zoom_from_scroll(0.0, -80.0), 0.0);
        assert!(zoom_from_scroll(1.0, 40.0) > 1.0);
    }

    #[test]
    fn corner_at_the_edge_fits_and_the_middle_zooms_in() {
        let crop = square_crop();
        let max_box = crop_rect(ClipCrop { zoom: 0.0, ..crop }, 1920.0, 1080.0);
        let fitted = zoom_from_corner(crop, max_box.x, max_box.y, 1920.0, 1080.0);
        assert!(
            fitted.zoom < 0.05,
            "edge handle should fit, got {}",
            fitted.zoom
        );
        let tight = zoom_from_corner(
            crop,
            max_box.x + max_box.w / 2.0,
            max_box.y + max_box.h / 2.0,
            1920.0,
            1080.0,
        );
        assert!(
            tight.zoom > 1.9,
            "center should reach the tightest zoom, got {}",
            tight.zoom
        );
    }

    #[test]
    fn window_scale_round_trips_through_zoom() {
        for zoom in [0.0, 0.25, 0.5, 1.0, 1.25, 1.5, 2.0] {
            close(zoom_for_window_scale(zoom_window_scale(zoom)), zoom);
        }
    }

    #[test]
    fn grabbing_the_current_corner_keeps_the_zoom() {
        for zoom in [0.0, 0.4, 1.0, 1.6, 2.0] {
            let crop = ClipCrop {
                ratio: CropRatio::Square,
                cx: 0.5,
                cy: 0.5,
                zoom,
            };
            let frame = crop_rect(crop, 1920.0, 1080.0);
            let held = zoom_from_corner(crop, frame.x, frame.y, 1920.0, 1080.0);
            assert!(
                (held.zoom - zoom).abs() < 0.02,
                "corner grab at {zoom} jumped to {}",
                held.zoom
            );
        }

        let panned = ClipCrop {
            ratio: CropRatio::R9x16,
            cx: 0.3,
            cy: 0.5,
            zoom: 1.0,
        };
        let frame = crop_rect(panned, 1920.0, 1080.0);
        let held = zoom_from_corner(panned, frame.x, frame.y, 1920.0, 1080.0);
        assert!(
            (held.zoom - 1.0).abs() < 0.02,
            "panned grab jumped to {}",
            held.zoom
        );

        let crop = ClipCrop {
            ratio: CropRatio::Square,
            cx: 0.5,
            cy: 0.5,
            zoom: 1.0,
        };
        let frame = crop_rect(crop, 1920.0, 1080.0);
        let cx = frame.x + frame.w / 2.0;
        let cy = frame.y + frame.h / 2.0;
        let tighter = zoom_from_corner(
            crop,
            frame.x + (cx - frame.x) * 0.2,
            frame.y + (cy - frame.y) * 0.2,
            1920.0,
            1080.0,
        );
        assert!(
            tighter.zoom > crop.zoom + 0.05,
            "inward drag should zoom in, got {}",
            tighter.zoom
        );
        let wider = zoom_from_corner(
            crop,
            frame.x - (cx - frame.x) * 0.15,
            frame.y - (cy - frame.y) * 0.15,
            1920.0,
            1080.0,
        );
        assert!(
            wider.zoom < crop.zoom - 0.05,
            "outward drag should zoom out, got {}",
            wider.zoom
        );
    }
}
