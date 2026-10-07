use base64::Engine;

pub const PREVIEW_PAD_MS: i64 = 12_000;
const PLAY_SNAP_EPS: f64 = 0.05;
const SCRUB_EPSILON_SEC: f64 = 0.04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewRange {
    pub file_start_ms: i64,
    pub file_end_ms: i64,
}

pub fn padded_preview_range(
    start_ms: i64,
    end_ms: i64,
    video_duration_ms: Option<i64>,
) -> PreviewRange {
    let file_start_ms = (start_ms - PREVIEW_PAD_MS).max(0);
    let mut file_end_ms = end_ms + PREVIEW_PAD_MS;
    if let Some(duration) = video_duration_ms {
        if duration > 0 {
            file_end_ms = file_end_ms.min(duration);
        }
    }
    if file_end_ms <= file_start_ms {
        file_end_ms = file_start_ms + (end_ms - start_ms).max(200);
    }
    PreviewRange {
        file_start_ms,
        file_end_ms,
    }
}

pub fn preview_covers_range(cover: PreviewRange, start_ms: i64, end_ms: i64) -> bool {
    start_ms >= cover.file_start_ms && end_ms <= cover.file_end_ms
}

pub fn preview_local_time(source_ms: i64, file_start_ms: i64) -> f64 {
    ((source_ms - file_start_ms) as f64 / 1000.0).max(0.0)
}

pub fn playback_time_on_play(current_time: f64, in_point: f64, out_point: f64) -> f64 {
    if !current_time.is_finite() {
        return in_point;
    }
    if current_time <= PLAY_SNAP_EPS && in_point > PLAY_SNAP_EPS {
        return in_point;
    }
    if current_time >= out_point - PLAY_SNAP_EPS {
        return in_point;
    }
    current_time
}

pub fn should_publish_playhead(seeking: bool, holding: bool) -> bool {
    !seeking && !holding
}

pub fn pointer_to_source_ms(
    client_x: f64,
    track_left: f64,
    track_width: f64,
    src_start: f64,
    src_end: f64,
) -> f64 {
    if !(track_width > 0.0) || !track_width.is_finite() {
        return src_start;
    }
    let x = (client_x - track_left).clamp(0.0, track_width);
    src_start + (x / track_width) * (src_end - src_start).max(1.0)
}

pub fn playback_ms_at(client_x: f64, left: f64, width: f64, start_ms: f64, end_ms: f64) -> f64 {
    let span = (end_ms - start_ms).max(0.0);
    if !(width > 0.0) || !client_x.is_finite() {
        return start_ms;
    }
    let ratio = ((client_x - left) / width).clamp(0.0, 1.0);
    start_ms + ratio * span
}

pub fn format_playback_clock(ms: f64) -> String {
    let safe = if ms.is_finite() { ms.max(0.0) } else { 0.0 };
    let total_seconds = (safe / 1000.0).floor() as i64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrubPlan {
    pub seek_to: Option<f64>,
    pub queued: Option<f64>,
}

pub fn plan_scrub(current_time: f64, requested: f64, busy: bool) -> ScrubPlan {
    plan_scrub_eps(current_time, requested, busy, SCRUB_EPSILON_SEC)
}

pub fn plan_scrub_eps(current_time: f64, requested: f64, busy: bool, epsilon: f64) -> ScrubPlan {
    if !requested.is_finite() {
        return ScrubPlan {
            seek_to: None,
            queued: None,
        };
    }
    if busy {
        return ScrubPlan {
            seek_to: None,
            queued: Some(requested),
        };
    }
    if !current_time.is_finite() || (current_time - requested).abs() > epsilon {
        return ScrubPlan {
            seek_to: Some(requested),
            queued: None,
        };
    }
    ScrubPlan {
        seek_to: None,
        queued: None,
    }
}

fn b64url_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(bytes)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn b64url_decode(token: &str) -> Option<Vec<u8>> {
    let padded = token.replace('-', "+").replace('_', "/");
    let pad = (4 - padded.len() % 4) % 4;
    let padded = format!("{padded}{}", "=".repeat(pad));
    base64::engine::general_purpose::STANDARD
        .decode(padded)
        .ok()
}

pub fn to_preview_src(file_path: &str) -> String {
    let posix = file_path.replace('\\', "/");
    format!("a1slice://preview/{}", b64url_encode(posix.as_bytes()))
}

pub fn path_from_preview_url(request_url: &str) -> Option<String> {
    let rest = request_url.split_once("://")?.1;
    let (path_and_host, query) = rest.split_once('?').unwrap_or((rest, ""));
    if let Some(query_path) = query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == "path").then_some(v)
    }) {
        return Some(percent_decode(query_path).replace('\\', "/"));
    }
    let token = path_and_host.split('/').next_back().unwrap_or("");
    if token.is_empty() || token == "preview" {
        return None;
    }
    let bytes = b64url_decode(token)?;
    Some(String::from_utf8(bytes).ok()?.replace('\\', "/"))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

pub fn preview_cache_key(
    video_path: &str,
    size: u64,
    mtime_ms: u128,
    file_start_ms: i64,
    file_end_ms: i64,
) -> String {
    let raw = format!(
        "{video_path}\0{size}\0{}\0{}\0{}",
        (mtime_ms as f64).round() as i64,
        file_start_ms,
        file_end_ms
    );
    let mut hash: u32 = 0x811c9dc5;
    for byte in raw.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{hash:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_url_round_trips_a_windows_path() {
        let file_path = r"C:\Users\water\Videos\sermon.mp4";
        let src = to_preview_src(file_path);
        assert!(src.starts_with("a1slice://preview/"));
        assert!(!src.contains('?'));
        assert_eq!(
            path_from_preview_url(&src).as_deref(),
            Some("C:/Users/water/Videos/sermon.mp4")
        );
    }

    #[test]
    fn reads_the_legacy_query_form() {
        let src = "a1slice://video?path=C%3A%2FUsers%2Fwater%2Fclip.mov";
        assert_eq!(
            path_from_preview_url(src).as_deref(),
            Some("C:/Users/water/clip.mov")
        );
    }

    #[test]
    fn pads_a_clip_window() {
        let range = padded_preview_range(60_000, 90_000, Some(600_000));
        assert_eq!(range.file_start_ms, 48_000);
        assert_eq!(range.file_end_ms, 102_000);
        assert!(preview_covers_range(range, 59_000, 91_000));
        assert!(!preview_covers_range(range, 40_000, 90_000));
    }

    #[test]
    fn local_time_maps_onto_the_preview() {
        assert_eq!(preview_local_time(57_600, 45_600), 12.0);
        assert_eq!(preview_local_time(1_000, 5_000), 0.0);
    }

    #[test]
    fn play_starts_at_the_in_point_from_zero() {
        let in_point = 12.0;
        let out_point = 58.3;
        assert_eq!(playback_time_on_play(0.0, in_point, out_point), in_point);
        assert_eq!(playback_time_on_play(0.02, in_point, out_point), in_point);
        assert_eq!(playback_time_on_play(20.0, in_point, out_point), 20.0);
        assert_eq!(playback_time_on_play(5.0, in_point, out_point), 5.0);
        assert_eq!(
            playback_time_on_play(out_point, in_point, out_point),
            in_point
        );
        assert_eq!(
            playback_time_on_play(out_point + 0.4, in_point, out_point),
            in_point
        );
        assert_eq!(playback_time_on_play(0.0, 0.0, 10.0), 0.0);
    }

    #[test]
    fn pointer_maps_the_bar() {
        let src_start = 45_600.0;
        let src_end = 115_900.0;
        let left = 100.0;
        let width = 400.0;
        assert_eq!(
            pointer_to_source_ms(left, left, width, src_start, src_end),
            src_start
        );
        assert_eq!(
            pointer_to_source_ms(left + width, left, width, src_start, src_end),
            src_end
        );
        assert_eq!(
            pointer_to_source_ms(left + width / 2.0, left, width, src_start, src_end),
            (src_start + src_end) / 2.0
        );
        assert_eq!(
            pointer_to_source_ms(left - 80.0, left, width, src_start, src_end),
            src_start
        );
        assert_eq!(
            pointer_to_source_ms(left + width + 80.0, left, width, src_start, src_end),
            src_end
        );
        assert_eq!(
            pointer_to_source_ms(140.0, left, 0.0, src_start, src_end),
            src_start
        );
        assert_eq!(
            pointer_to_source_ms(140.0, left, f64::NAN, src_start, src_end),
            src_start
        );
    }

    #[test]
    fn playhead_publish_flag() {
        assert!(!should_publish_playhead(true, false));
        assert!(!should_publish_playhead(false, true));
        assert!(should_publish_playhead(false, false));
    }

    #[test]
    fn clock_and_bar_click() {
        assert_eq!(format_playback_clock(0.0), "0:00");
        assert_eq!(format_playback_clock(65_000.0), "1:05");
        assert_eq!(format_playback_clock(3_723_000.0), "1:02:03");
        assert_eq!(playback_ms_at(0.0, 0.0, 200.0, 0.0, 10_000.0), 0.0);
        assert_eq!(playback_ms_at(100.0, 0.0, 200.0, 0.0, 10_000.0), 5_000.0);
        assert_eq!(playback_ms_at(200.0, 0.0, 200.0, 0.0, 10_000.0), 10_000.0);
        assert_eq!(playback_ms_at(-20.0, 0.0, 200.0, 1_000.0, 3_000.0), 1_000.0);
        assert_eq!(playback_ms_at(999.0, 0.0, 200.0, 1_000.0, 3_000.0), 3_000.0);
    }

    #[test]
    fn scrub_plan() {
        assert_eq!(
            plan_scrub(0.0, 12.5, false),
            ScrubPlan {
                seek_to: Some(12.5),
                queued: None
            }
        );
        assert_eq!(
            plan_scrub(1.0, 4.0, true),
            ScrubPlan {
                seek_to: None,
                queued: Some(4.0)
            }
        );
        assert_eq!(
            plan_scrub(1.0, 9.0, true),
            ScrubPlan {
                seek_to: None,
                queued: Some(9.0)
            }
        );
        assert_eq!(
            plan_scrub(5.0, 5.02, false),
            ScrubPlan {
                seek_to: None,
                queued: None
            }
        );
        let queued = plan_scrub(1.0, 9.0, true).queued.unwrap();
        assert_eq!(
            plan_scrub(1.0, queued, false),
            ScrubPlan {
                seek_to: Some(9.0),
                queued: None
            }
        );
    }
}
