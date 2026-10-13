//! In-process video decoding, frame grabbing, and playback streaming using video-rs.

use a1slice_core::types::Ms;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Once};
use std::time::Instant;
use video_rs::decode::DecoderBuilder;
use video_rs::Resize;

static VIDEO_RS_INIT: Once = Once::new();

pub fn ensure_video_init() {
    VIDEO_RS_INIT.call_once(|| {
        let _ = video_rs::init();
    });
}

#[inline]
pub fn rgb_to_rgba(rgb: &[u8], width: usize, height: usize) -> Vec<u8> {
    let pixel_count = width * height;
    let mut rgba = vec![255u8; pixel_count * 4];
    let chunks = rgb.chunks_exact(3);
    for (src, dst) in chunks.zip(rgba.chunks_exact_mut(4)) {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
    }
    rgba
}

pub fn probe(video: &Path) -> Result<(Ms, u32, u32), String> {
    ensure_video_init();
    if let Ok(decoder) = video_rs::decode::Decoder::new(video) {
        let (w, h) = decoder.size();
        if w > 0 && h > 0 {
            if let Ok(duration) = decoder.duration() {
                let ms = (duration.as_secs_f64() * 1000.0).round() as Ms;
                if ms > 0 {
                    return Ok((ms, w, h));
                }
            }
        }
    }
    super::process::probe_cli(video)
}

pub fn probe_size(video: &Path) -> Result<(u32, u32), String> {
    ensure_video_init();
    if let Ok(decoder) = video_rs::decode::Decoder::new(video) {
        let (w, h) = decoder.size();
        if w > 0 && h > 0 {
            return Ok((w, h));
        }
    }
    super::process::probe_size_cli(video)
}

#[allow(dead_code)]
pub fn grab_frame(video: &Path, at_ms: Ms, max_w: u32) -> Result<(u32, u32, Vec<u8>), String> {
    grab_frame_sized(video, at_ms, max_w, None)
}

pub fn grab_frame_sized(
    video: &Path,
    at_ms: Ms,
    max_w: u32,
    known_size: Option<(u32, u32)>,
) -> Result<(u32, u32, Vec<u8>), String> {
    ensure_video_init();
    let (width, height) = match known_size {
        Some((w, h)) if w > 0 && h > 0 => (w, h),
        _ => probe_size(video)?,
    };
    if width == 0 || height == 0 {
        return Err("This file has no picture.".into());
    }
    let scale_w = max_w.min(width).max(2) / 2 * 2;
    let scale_h = ((height as f64) * (scale_w as f64) / (width as f64))
        .round()
        .max(2.0) as u32
        / 2
        * 2;

    let mut builder = DecoderBuilder::new(video);
    if scale_w < width && scale_h < height {
        builder = builder.with_resize(Resize::Fit(scale_w, scale_h));
    }
    if let Ok(mut decoder) = builder.build() {
        let target_ms = at_ms.max(0);
        let _ = decoder.seek(target_ms);
        if let Ok(raw) = decoder.decode_raw() {
            let w = raw.width();
            let h = raw.height();
            let rgb = raw.data(0);
            let rgba = rgb_to_rgba(rgb, w as usize, h as usize);
            return Ok((w, h, rgba));
        }
    }
    super::process::grab_frame_sized_cli(video, at_ms, max_w, known_size)
}

pub struct VideoStream {
    pub frame_rx: Receiver<(u32, u32, Vec<u8>, i64)>,
    cancel: Arc<AtomicBool>,
}

impl Drop for VideoStream {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl VideoStream {
    pub fn stop(self) {
        // Drop handles cleanup
    }
}

pub fn start_video_stream(
    video: &Path,
    at_ms: Ms,
    max_w: u32,
    size: (u32, u32),
) -> Result<VideoStream, String> {
    ensure_video_init();
    let (width, height) = if size.0 > 0 && size.1 > 0 {
        size
    } else {
        probe_size(video)?
    };
    if width == 0 || height == 0 {
        return Err("This file has no picture.".into());
    }

    let scale_w = max_w.min(width).max(2) / 2 * 2;
    let scale_h = ((height as f64) * (scale_w as f64) / (width as f64))
        .round()
        .max(2.0) as u32
        / 2
        * 2;

    let path_buf = video.to_path_buf();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_flag = cancel.clone();
    let (tx, rx) = sync_channel(2);
    let start_wall = Instant::now();

    std::thread::spawn(move || {
        let mut builder = DecoderBuilder::new(path_buf.as_path());
        if scale_w < width && scale_h < height {
            builder = builder.with_resize(Resize::Fit(scale_w, scale_h));
        }

        let mut decoder = match builder.build() {
            Ok(d) => d,
            Err(_) => return,
        };

        let (time_base_num, time_base_den) = {
            let tb = decoder.time_base();
            (tb.numerator() as f64, tb.denominator() as f64)
        };

        let start_seek_ms = at_ms.max(0);
        let _ = decoder.seek(start_seek_ms.saturating_sub(1000));

        let mut ready_to_play = false;
        let mut frame_count = 0i64;

        while !cancel_flag.load(Ordering::Relaxed) {
            let raw = match decoder.decode_raw() {
                Ok(frame) => frame,
                Err(_) => break,
            };

            let pts_ms = raw
                .pts()
                .map(|pts| (pts as f64 * (time_base_num / time_base_den) * 1000.0).round() as i64)
                .unwrap_or(start_seek_ms + (frame_count * 33));

            if !ready_to_play {
                if pts_ms < start_seek_ms {
                    continue;
                }
                ready_to_play = true;
            }

            let elapsed_stream_ms = (pts_ms - start_seek_ms).max(0);
            let target_duration = std::time::Duration::from_millis(elapsed_stream_ms as u64);
            let wall_elapsed = start_wall.elapsed();

            if wall_elapsed < target_duration {
                let to_sleep = target_duration - wall_elapsed;
                if to_sleep.as_millis() > 30 {
                    let end = Instant::now() + to_sleep;
                    while Instant::now() < end {
                        if cancel_flag.load(Ordering::Relaxed) {
                            return;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                } else {
                    std::thread::sleep(to_sleep);
                }
            }

            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            let rgb = raw.data(0);
            let rgba = rgb_to_rgba(rgb, raw.width() as usize, raw.height() as usize);

            if tx.send((raw.width(), raw.height(), rgba, pts_ms)).is_err() {
                break;
            }
            frame_count += 1;
        }
    });

    Ok(VideoStream {
        frame_rx: rx,
        cancel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_to_rgba_converts_correctly() {
        let rgb = [255, 0, 128, 10, 20, 30];
        let rgba = rgb_to_rgba(&rgb, 2, 1);
        assert_eq!(rgba, vec![255, 0, 128, 255, 10, 20, 30, 255]);
    }

    #[test]
    fn test_por_que_reunimos_file_if_present() {
        let mov = std::path::Path::new(r"C:\Users\water\Videos\por-que-reunimos.mov");
        if mov.is_file() {
            let (ms, w, h) = probe(mov).expect("probe should succeed");
            assert_eq!((w, h), (1920, 1080));
            assert!(ms > 0);

            let (fw, fh, pixels) = grab_frame(mov, 0, 960).expect("grab_frame should succeed");
            assert_eq!((fw, fh), (960, 540));
            assert_eq!(pixels.len(), 960 * 540 * 4);

            let start = std::time::Instant::now();
            let (sfw, sfh, spixels) = grab_frame_sized(mov, 1000, 960, Some((w, h)))
                .expect("grab_frame_sized should succeed");
            let elapsed = start.elapsed();
            assert_eq!((sfw, sfh), (960, 540));
            assert_eq!(spixels.len(), 960 * 540 * 4);
            println!("video-rs grab_frame_sized elapsed: {elapsed:?}");
        }
    }
}
