use crate::crop::clamp;
use crate::types::{ClipSegment, Ms, TranscriptSegment};

const PAUSE_MIN_MS: Ms = 400;
const PRE_ROLL_MS: Ms = 250;
const POST_ROLL_MS: Ms = 250;
const PAUSE_EDGE_PAD_MS: Ms = 40;

pub const CLIP_VIEW_PAD_MS: Ms = 60_000;
pub const FINE_DRAG_PX: f64 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipViewWindow {
    pub view_start: Ms,
    pub view_end: Ms,
}

pub fn swap_inverted_times(mut clip: ClipSegment) -> ClipSegment {
    if clip.start_ms > clip.end_ms {
        std::mem::swap(&mut clip.start_ms, &mut clip.end_ms);
    }
    clip
}

fn find_segment_index_at(ms: Ms, segments: &[TranscriptSegment], prefer_start: bool) -> isize {
    if segments.is_empty() {
        return -1;
    }
    for (i, segment) in segments.iter().enumerate() {
        if ms >= segment.start_ms && ms <= segment.end_ms {
            return i as isize;
        }
    }
    if prefer_start {
        for (i, segment) in segments.iter().enumerate() {
            if segment.start_ms >= ms {
                return i as isize;
            }
        }
        return (segments.len() - 1) as isize;
    }
    for (i, segment) in segments.iter().enumerate().rev() {
        if segment.end_ms <= ms {
            return i as isize;
        }
    }
    0
}

pub fn snap_clip_to_segments(mut clip: ClipSegment, segments: &[TranscriptSegment]) -> ClipSegment {
    if segments.is_empty() {
        return clip;
    }
    let start_idx = find_segment_index_at(clip.start_ms, segments, true) as usize;
    let end_idx = find_segment_index_at(clip.end_ms, segments, false) as usize;
    let start_ms = segments[start_idx].start_ms;
    let end_ms = segments[end_idx].end_ms;
    if end_ms <= start_ms {
        clip.start_ms = start_ms;
        clip.end_ms = end_ms.max(segments[start_idx].end_ms);
        return clip;
    }
    clip.start_ms = start_ms;
    clip.end_ms = end_ms;
    clip
}

pub fn snap_to_pause(mut clip: ClipSegment, segments: &[TranscriptSegment]) -> ClipSegment {
    if segments.is_empty() {
        return clip;
    }
    let start_exact = segments.iter().position(|s| s.start_ms == clip.start_ms);
    let end_exact = segments.iter().position(|s| s.end_ms == clip.end_ms);
    let start_idx = start_exact
        .unwrap_or_else(|| find_segment_index_at(clip.start_ms, segments, true) as usize);
    let end_idx =
        end_exact.unwrap_or_else(|| find_segment_index_at(clip.end_ms, segments, false) as usize);

    if start_idx > 0 {
        let prev = &segments[start_idx - 1];
        let cur = &segments[start_idx];
        let gap = cur.start_ms - prev.end_ms;
        if gap >= PAUSE_MIN_MS {
            clip.start_ms = (prev.end_ms + PAUSE_EDGE_PAD_MS).max(cur.start_ms - PRE_ROLL_MS);
        }
    }
    if end_idx < segments.len() - 1 {
        let cur = &segments[end_idx];
        let next = &segments[end_idx + 1];
        let gap = next.start_ms - cur.end_ms;
        if gap >= PAUSE_MIN_MS {
            clip.end_ms = (next.start_ms - PAUSE_EDGE_PAD_MS).min(cur.end_ms + POST_ROLL_MS);
        }
    }
    clip
}

pub fn trim_edge(edge: &str, pointer_ms: f64, start_ms: Ms, end_ms: Ms, max_ms: f64) -> (Ms, Ms) {
    let limit = if max_ms.is_finite() && max_ms > 0.0 {
        max_ms
    } else {
        end_ms.max(start_ms) as f64
    };
    if edge == "start" {
        let start = clamp(pointer_ms, 0.0, (end_ms as f64 - 500.0).max(0.0));
        (start.round() as Ms, end_ms)
    } else {
        let end = clamp(pointer_ms, limit.min(start_ms as f64 + 500.0), limit);
        (start_ms, end.round() as Ms)
    }
}

fn video_limit(max_ms: f64, start_ms: Ms, end_ms: Ms) -> f64 {
    if max_ms.is_finite() && max_ms > 0.0 {
        max_ms
    } else {
        (end_ms.max(start_ms).max(1)) as f64
    }
}

pub fn clip_view_window(start_ms: Ms, end_ms: Ms, max_ms: f64, pad_ms: Ms) -> ClipViewWindow {
    let limit = video_limit(max_ms, start_ms, end_ms);
    let start = clamp(start_ms as f64, 0.0, limit);
    let end = clamp((end_ms as f64).max(start), 0.0, limit);
    let view_start = (start - pad_ms as f64).max(0.0);
    let mut view_end = (end + pad_ms as f64).min(limit);
    if view_end - view_start < 1.0 {
        view_end = (view_start + 1.0).min(limit);
    }
    ClipViewWindow {
        view_start: view_start.round() as Ms,
        view_end: view_end.round() as Ms,
    }
}

pub fn expand_view_to_fit(
    view: ClipViewWindow,
    start_ms: Ms,
    end_ms: Ms,
    max_ms: f64,
    pad_ms: Ms,
) -> ClipViewWindow {
    let limit = video_limit(max_ms, start_ms, end_ms.max(view.view_end));
    let mut view_start = view.view_start as f64;
    let mut view_end = view.view_end as f64;
    if (start_ms as f64) < view_start {
        view_start = (start_ms as f64 - pad_ms as f64).max(0.0);
    }
    if (end_ms as f64) > view_end {
        view_end = (end_ms as f64 + pad_ms as f64).min(limit);
    }
    if view_end - view_start < 1.0 {
        view_end = (view_start + 1.0).min(limit);
    }
    ClipViewWindow {
        view_start: view_start.round() as Ms,
        view_end: view_end.round() as Ms,
    }
}

pub fn seconds_from_drag(dx_px: f64, px_per_second: f64) -> i64 {
    if !dx_px.is_finite() || px_per_second <= 0.0 {
        return 0;
    }
    (dx_px / px_per_second).trunc() as i64
}

pub fn nudge_edge(start_ms: Ms, end_ms: Ms, edge: &str, delta_ms: f64, max_ms: f64) -> (Ms, Ms) {
    let limit = if max_ms.is_finite() && max_ms > 0.0 {
        max_ms
    } else {
        end_ms.max(start_ms) as f64
    };
    let mut start = start_ms as f64 + if edge == "start" { delta_ms } else { 0.0 };
    let mut end = end_ms as f64 + if edge == "end" { delta_ms } else { 0.0 };
    start = clamp(start, 0.0, limit);
    end = clamp(end, 0.0, limit);
    if end - start < 500.0 {
        if edge == "start" {
            start = clamp(end - 500.0, 0.0, limit);
        } else {
            end = clamp(start + 500.0, 0.0, limit);
        }
    }
    if end - start < 500.0 {
        if edge == "start" {
            end = clamp(start + 500.0, 0.0, limit);
        } else {
            start = clamp(end - 500.0, 0.0, limit);
        }
    }
    (start.round() as Ms, end.round() as Ms)
}

pub fn refine_clip_bounds(clip: ClipSegment, segments: &[TranscriptSegment]) -> ClipSegment {
    let mut result = swap_inverted_times(clip);
    if segments.is_empty() {
        result.start_ms = result.start_ms.max(0);
        result.end_ms = result.end_ms.max(0);
        return result;
    }
    let max_ms = segments.last().map(|s| s.end_ms).unwrap_or(0);
    result.start_ms = result.start_ms.clamp(0, max_ms);
    result.end_ms = result.end_ms.clamp(0, max_ms);
    result = swap_inverted_times(result);
    result = snap_clip_to_segments(result, segments);
    snap_to_pause(result, segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ClipCategory;

    fn seg(start: Ms, end: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms: start,
            end_ms: end,
            text: text.into(),
        }
    }

    fn clip(title: &str, start: Ms, end: Ms) -> ClipSegment {
        ClipSegment {
            title: title.into(),
            start_ms: start,
            end_ms: end,
            category: None,
            topic: None,
            crop: None,
            approved: None,
        }
    }

    #[test]
    fn swaps_when_start_is_after_end() {
        let got = swap_inverted_times(clip("A", 9000, 1000));
        assert_eq!(got.start_ms, 1000);
        assert_eq!(got.end_ms, 9000);
        assert_eq!(got.title, "A");
    }

    #[test]
    fn leaves_ordered_clips_unchanged() {
        let clip = clip("A", 0, 5000);
        assert_eq!(swap_inverted_times(clip.clone()), clip);
    }

    fn sample() -> Vec<TranscriptSegment> {
        vec![
            seg(0, 4000, "S0"),
            seg(4500, 9000, "S1"),
            seg(9200, 15000, "S2"),
            seg(16000, 22000, "S3"),
        ]
    }

    #[test]
    fn snaps_to_containing_segments() {
        let snapped = snap_clip_to_segments(clip("A", 1200, 14000), &sample());
        assert_eq!(snapped.start_ms, 0);
        assert_eq!(snapped.end_ms, 15000);
    }

    #[test]
    fn snaps_a_gap_to_the_next_segment_start() {
        let snapped = snap_clip_to_segments(clip("A", 4100, 15500), &sample());
        assert_eq!(snapped.start_ms, 4500);
        assert_eq!(snapped.end_ms, 15000);
    }

    #[test]
    fn snap_is_unchanged_without_segments() {
        let clip = clip("A", 100, 200);
        assert_eq!(snap_clip_to_segments(clip.clone(), &[]), clip);
    }

    #[test]
    fn pre_roll_into_a_pause() {
        let segments = vec![
            seg(0, 4000, ""),
            seg(4800, 9000, ""),
            seg(9200, 15000, ""),
            seg(17000, 22000, ""),
        ];
        let result = snap_to_pause(clip("A", 4800, 9000), &segments);
        assert!(result.start_ms < 4800);
        assert!(result.start_ms >= 4000);
        assert_eq!(result.end_ms, 9000);
    }

    #[test]
    fn tiny_gap_does_not_eat_previous_speech() {
        let segments = vec![
            seg(0, 4000, ""),
            seg(4800, 9000, ""),
            seg(9200, 15000, ""),
            seg(17000, 22000, ""),
        ];
        let result = snap_to_pause(clip("A", 9200, 15000), &segments);
        assert_eq!(result.start_ms, 9200);
        assert!(result.end_ms > 15000);
        assert!(result.end_ms < 17000);
    }

    #[test]
    fn trim_moves_one_edge() {
        assert_eq!(trim_edge("end", 9000.0, 1000, 4000, 20000.0), (1000, 9000));
        assert_eq!(trim_edge("start", 500.0, 1000, 4000, 20000.0), (500, 4000));
        assert_eq!(trim_edge("end", 1000.0, 1000, 4000, 20000.0), (1000, 1500));
        assert_eq!(
            trim_edge("start", 9000.0, 1000, 4000, 20000.0),
            (3500, 4000)
        );
    }

    #[test]
    fn view_window_pads_and_clamps() {
        let hour = 3_600_000;
        assert_eq!(
            clip_view_window(hour, hour + 21_000, (3 * hour) as f64, CLIP_VIEW_PAD_MS),
            ClipViewWindow {
                view_start: hour - 60_000,
                view_end: hour + 21_000 + 60_000
            }
        );
        assert_eq!(
            clip_view_window(5_000, 26_000, 600_000.0, CLIP_VIEW_PAD_MS),
            ClipViewWindow {
                view_start: 0,
                view_end: 86_000
            }
        );
        assert_eq!(
            clip_view_window(10_000, 40_000, 600_000.0, CLIP_VIEW_PAD_MS),
            ClipViewWindow {
                view_start: 0,
                view_end: 100_000
            }
        );
    }

    #[test]
    fn expand_grows_only_the_side_that_moved() {
        let view = ClipViewWindow {
            view_start: 1_000_000,
            view_end: 1_200_000,
        };
        assert_eq!(
            expand_view_to_fit(view, 1_050_000, 1_250_000, 3_600_000.0, CLIP_VIEW_PAD_MS),
            ClipViewWindow {
                view_start: 1_000_000,
                view_end: 1_250_000 + 60_000
            }
        );
        let inside = ClipViewWindow {
            view_start: 0,
            view_end: 180_000,
        };
        assert_eq!(
            expand_view_to_fit(inside, 10_000, 40_000, 600_000.0, CLIP_VIEW_PAD_MS),
            inside
        );
    }

    #[test]
    fn drag_counts_whole_seconds() {
        assert_eq!(seconds_from_drag(23.0, FINE_DRAG_PX), 0);
        assert_eq!(seconds_from_drag(24.0, FINE_DRAG_PX), 1);
        assert_eq!(seconds_from_drag(-24.0, FINE_DRAG_PX), -1);
        assert_eq!(seconds_from_drag(-47.0, FINE_DRAG_PX), -1);
        assert_eq!(seconds_from_drag(48.0, FINE_DRAG_PX), 2);
    }

    #[test]
    fn nudge_clamps_and_keeps_a_minimum() {
        assert_eq!(nudge_edge(500, 4000, "start", -1000.0, 20000.0), (0, 4000));
        assert_eq!(nudge_edge(0, 800, "end", -1000.0, 20000.0), (0, 500));
        assert_eq!(nudge_edge(1000, 2500, "end", 1000.0, 3000.0), (1000, 3000));
    }

    #[test]
    fn refine_swaps_then_snaps_and_keeps_fields() {
        let segments = sample();
        let result = refine_clip_bounds(clip("A", 14000, 1200), &segments);
        assert!(result.start_ms < result.end_ms);
        assert!(result.start_ms <= 4000);
        assert!(result.end_ms >= 15000);

        let clamped = refine_clip_bounds(clip("A", -500, 999_999), &segments);
        assert!(clamped.start_ms >= 0);
        assert!(clamped.end_ms <= 22000);

        let mut kept = clip("Keep", 5000, 8000);
        kept.category = Some(ClipCategory::Standalone);
        kept.topic = Some("T".into());
        let kept = refine_clip_bounds(kept, &segments);
        assert_eq!(kept.title, "Keep");
        assert_eq!(kept.category, Some(ClipCategory::Standalone));
        assert_eq!(kept.topic.as_deref(), Some("T"));
    }
}
