//! Whisper timestamp parsing and binary selection.
//!
//! `os` and `arch` accept Node's `platform()` / `arch()` (`win32`, `darwin`, `x64`, `arm64`)
//! and Rust's `std::env::consts` (`windows`, `macos`, `linux`, `x86_64`, `aarch64`).
//! The choice itself only looks at the names you pass in. It does not read the disk.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::types::{Ms, TranscriptSegment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperChunk {
    pub offset_ms: Ms,
    pub duration_ms: Ms,
    pub segments: Vec<TranscriptSegment>,
}

pub fn parse_timestamp(ts: &str) -> Ms {
    let normalized = ts.replace(',', ".");
    let mut parts = normalized.split(':');
    let hours = parse_js_int(parts.next().unwrap_or("0"));
    let minutes = parse_js_int(parts.next().unwrap_or("0"));
    let seconds_part = parts.next().unwrap_or("0");
    let mut sec_parts = seconds_part.split('.');
    let seconds = parse_js_int(sec_parts.next().unwrap_or("0"));
    let frac = sec_parts.next().unwrap_or("0");
    let millis = millis_from_frac(frac);
    hours * 3_600_000 + minutes * 60_000 + seconds * 1000 + millis
}

fn parse_js_int(text: &str) -> i64 {
    let text = text.trim();
    let (sign, rest) = if let Some(rest) = text.strip_prefix('-') {
        (-1, rest)
    } else if let Some(rest) = text.strip_prefix('+') {
        (1, rest)
    } else {
        (1, text)
    };
    let mut value: i64 = 0;
    let mut any = false;
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            any = true;
            value = value
                .saturating_mul(10)
                .saturating_add((ch as u8 - b'0') as i64);
        } else {
            break;
        }
    }
    if any {
        sign * value
    } else {
        0
    }
}

fn millis_from_frac(frac: &str) -> i64 {
    let mut padded: String = frac.chars().take(3).collect();
    while padded.chars().count() < 3 {
        padded.push('0');
    }
    parse_js_int(&padded)
}

pub fn merge_chunk_segments(chunks: &[WhisperChunk], overlap_ms: i64) -> Vec<TranscriptSegment> {
    let half = overlap_ms as f64 / 2.0;
    let last = chunks.len().saturating_sub(1);
    let mut merged = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let keep_start = if i == 0 {
            f64::NEG_INFINITY
        } else {
            chunk.offset_ms as f64 + half
        };
        let keep_end = if i == last {
            f64::INFINITY
        } else {
            chunk.offset_ms as f64 + chunk.duration_ms as f64 - half
        };
        for seg in &chunk.segments {
            let start_ms = seg.start_ms + chunk.offset_ms;
            let end_ms = seg.end_ms + chunk.offset_ms;
            if start_ms as f64 >= keep_start && (start_ms as f64) < keep_end {
                merged.push(TranscriptSegment {
                    start_ms,
                    end_ms,
                    text: seg.text.clone(),
                });
            }
        }
    }
    merged.sort_by_key(|segment| segment.start_ms);
    merged
}

#[derive(Debug, Deserialize)]
struct WhisperFile {
    transcription: Vec<WhisperEntry>,
}

#[derive(Debug, Deserialize)]
struct WhisperEntry {
    timestamps: WhisperTimestamps,
    text: String,
}

#[derive(Debug, Deserialize)]
struct WhisperTimestamps {
    from: String,
    to: String,
}

pub fn parse_whisper_json(json: &str) -> Result<Vec<TranscriptSegment>, serde_json::Error> {
    let data: WhisperFile = serde_json::from_str(json)?;
    Ok(data
        .transcription
        .into_iter()
        .map(|entry| TranscriptSegment {
            start_ms: parse_timestamp(&entry.timestamps.from),
            end_ms: parse_timestamp(&entry.timestamps.to),
            text: entry.text,
        })
        .collect())
}

fn is_windows(os: &str) -> bool {
    os == "win32" || os.eq_ignore_ascii_case("windows")
}

fn is_macos(os: &str) -> bool {
    os == "darwin" || os.eq_ignore_ascii_case("macos")
}

fn is_arm64(arch: &str) -> bool {
    arch == "arm64" || arch == "aarch64"
}

/// `(gpu_file_name, cpu_file_name)`. GPU is the `-gpu` suffix of the CPU name.
pub fn whisper_binary_names(os: &str, arch: &str) -> (String, String) {
    let base = if is_windows(os) {
        "whisper-cli-win-x64".to_string()
    } else if is_macos(os) {
        if is_arm64(arch) {
            "whisper-cli-mac-arm64".to_string()
        } else {
            "whisper-cli-mac-x64".to_string()
        }
    } else if is_arm64(arch) {
        "whisper-cli-linux-arm64".to_string()
    } else {
        "whisper-cli-linux-x64".to_string()
    };
    let ext = if is_windows(os) { ".exe" } else { "" };
    (format!("{base}-gpu{ext}"), format!("{base}{ext}"))
}

/// GPU is preferred when that file name is in `present`. Otherwise the CPU name.
pub fn choose_whisper_binary_name(os: &str, arch: &str, present: &[&str]) -> String {
    let (gpu, cpu) = whisper_binary_names(os, arch);
    if present.iter().any(|name| *name == gpu) {
        gpu
    } else {
        cpu
    }
}

/// Search `locations` in order. Each entry is a directory plus the file names listed in it.
/// A GPU file in a later directory wins over a CPU file in an earlier one.
/// When neither name is listed, the result is `fallback_dir` joined with the CPU name.
pub fn choose_whisper_binary_path(
    os: &str,
    arch: &str,
    locations: &[(&Path, &[&str])],
    fallback_dir: &Path,
) -> PathBuf {
    let (gpu, cpu) = whisper_binary_names(os, arch);
    for name in [&gpu, &cpu] {
        for (dir, files) in locations {
            if files.iter().any(|file| file == name) {
                return dir.join(name);
            }
        }
    }
    fallback_dir.join(cpu)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start_ms: Ms, end_ms: Ms, text: &str) -> TranscriptSegment {
        TranscriptSegment {
            start_ms,
            end_ms,
            text: text.into(),
        }
    }

    #[test]
    fn parse_timestamp_zero() {
        assert_eq!(parse_timestamp("00:00:00.000"), 0);
    }

    #[test]
    fn parse_timestamp_seconds_and_millis() {
        assert_eq!(parse_timestamp("00:00:05.123"), 5123);
    }

    #[test]
    fn parse_timestamp_minutes() {
        assert_eq!(parse_timestamp("00:01:30.000"), 90000);
    }

    #[test]
    fn parse_timestamp_hours() {
        assert_eq!(parse_timestamp("01:01:01.001"), 3661001);
    }

    #[test]
    fn parse_timestamp_comma_separator() {
        assert_eq!(parse_timestamp("00:00:05,123"), 5123);
    }

    #[test]
    fn parse_timestamp_missing_millis() {
        assert_eq!(parse_timestamp("00:01:00.0"), 60000);
    }

    #[test]
    fn parse_whisper_json_output() {
        let json = r#"{
            "transcription": [
                {"timestamps": {"from": "00:00:00.000", "to": "00:00:05.000"}, "text": " Hello world"},
                {"timestamps": {"from": "00:00:05.000", "to": "00:00:10.500"}, "text": " How are you"}
            ]
        }"#;
        let result = parse_whisper_json(json).unwrap();
        assert_eq!(
            result,
            vec![
                seg(0, 5000, " Hello world"),
                seg(5000, 10500, " How are you")
            ]
        );
    }

    #[test]
    fn parse_whisper_json_empty() {
        let result = parse_whisper_json(r#"{"transcription":[]}"#).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn merge_keeps_the_earlier_chunk_in_the_first_half_of_the_overlap() {
        let merged = merge_chunk_segments(
            &[
                WhisperChunk {
                    offset_ms: 0,
                    duration_ms: 180000,
                    segments: vec![seg(0, 2000, "start"), seg(170000, 172000, "from-first")],
                },
                WhisperChunk {
                    offset_ms: 165000,
                    duration_ms: 180000,
                    segments: vec![
                        seg(2000, 4000, "from-second-early"),
                        seg(20000, 22000, "from-second"),
                    ],
                },
            ],
            15000,
        );
        let texts: Vec<&str> = merged.iter().map(|segment| segment.text.as_str()).collect();
        assert!(texts.contains(&"start"));
        assert!(texts.contains(&"from-first"));
        assert!(texts.contains(&"from-second"));
        assert!(!texts.contains(&"from-second-early"));
    }

    #[test]
    fn merge_applies_chunk_offsets() {
        let merged = merge_chunk_segments(
            &[
                WhisperChunk {
                    offset_ms: 0,
                    duration_ms: 10000,
                    segments: vec![seg(1000, 2000, "a")],
                },
                WhisperChunk {
                    offset_ms: 8000,
                    duration_ms: 10000,
                    segments: vec![seg(3000, 4000, "b")],
                },
            ],
            2000,
        );
        assert_eq!(
            merged
                .iter()
                .find(|segment| segment.text == "a")
                .unwrap()
                .start_ms,
            1000
        );
        assert_eq!(
            merged
                .iter()
                .find(|segment| segment.text == "b")
                .unwrap()
                .start_ms,
            11000
        );
    }

    #[test]
    fn binary_names_match_each_platform() {
        assert_eq!(
            whisper_binary_names("win32", "x64"),
            (
                "whisper-cli-win-x64-gpu.exe".into(),
                "whisper-cli-win-x64.exe".into()
            )
        );
        assert_eq!(
            whisper_binary_names("windows", "x86_64"),
            (
                "whisper-cli-win-x64-gpu.exe".into(),
                "whisper-cli-win-x64.exe".into()
            )
        );
        assert_eq!(
            whisper_binary_names("darwin", "arm64"),
            (
                "whisper-cli-mac-arm64-gpu".into(),
                "whisper-cli-mac-arm64".into()
            )
        );
        assert_eq!(
            whisper_binary_names("macos", "aarch64"),
            (
                "whisper-cli-mac-arm64-gpu".into(),
                "whisper-cli-mac-arm64".into()
            )
        );
        assert_eq!(
            whisper_binary_names("darwin", "x64"),
            (
                "whisper-cli-mac-x64-gpu".into(),
                "whisper-cli-mac-x64".into()
            )
        );
        assert_eq!(
            whisper_binary_names("linux", "x64"),
            (
                "whisper-cli-linux-x64-gpu".into(),
                "whisper-cli-linux-x64".into()
            )
        );
        assert_eq!(
            whisper_binary_names("linux", "arm64"),
            (
                "whisper-cli-linux-arm64-gpu".into(),
                "whisper-cli-linux-arm64".into()
            )
        );
    }

    #[test]
    fn choose_name_prefers_gpu_when_listed() {
        let present = [
            "whisper-cli-mac-arm64",
            "whisper-cli-mac-arm64-gpu",
            "notes.txt",
        ];
        assert_eq!(
            choose_whisper_binary_name("darwin", "arm64", &present),
            "whisper-cli-mac-arm64-gpu"
        );
        assert_eq!(
            choose_whisper_binary_name("darwin", "arm64", &["whisper-cli-mac-arm64"]),
            "whisper-cli-mac-arm64"
        );
        assert_eq!(
            choose_whisper_binary_name("linux", "x64", &[]),
            "whisper-cli-linux-x64"
        );
    }

    #[test]
    fn choose_path_uses_a_temp_directory_listing() {
        let root = tempfile::tempdir().unwrap();
        let prod = root.path().join("prod");
        let dev = root.path().join("dev");
        std::fs::create_dir(&prod).unwrap();
        std::fs::create_dir(&dev).unwrap();
        std::fs::write(prod.join("whisper-cli-linux-x64"), b"").unwrap();
        std::fs::write(dev.join("whisper-cli-linux-x64-gpu"), b"").unwrap();
        std::fs::write(dev.join("whisper-cli-linux-x64"), b"").unwrap();

        let prod_files = list_names(&prod);
        let dev_files = list_names(&dev);
        let prod_refs: Vec<&str> = prod_files.iter().map(String::as_str).collect();
        let dev_refs: Vec<&str> = dev_files.iter().map(String::as_str).collect();
        let locations = [
            (prod.as_path(), prod_refs.as_slice()),
            (dev.as_path(), dev_refs.as_slice()),
        ];
        let chosen = choose_whisper_binary_path("linux", "x64", &locations, dev.as_path());
        assert_eq!(chosen, dev.join("whisper-cli-linux-x64-gpu"));

        let cpu_only = [(prod.as_path(), prod_refs.as_slice())];
        let chosen = choose_whisper_binary_path("linux", "x64", &cpu_only, dev.as_path());
        assert_eq!(chosen, prod.join("whisper-cli-linux-x64"));

        let chosen = choose_whisper_binary_path("linux", "arm64", &[], dev.as_path());
        assert_eq!(chosen, dev.join("whisper-cli-linux-arm64"));
    }

    fn list_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}
