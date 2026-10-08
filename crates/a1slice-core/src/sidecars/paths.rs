//! Where transcripts, clips, models, and previews are stored.

use std::path::{Path, PathBuf};

pub fn a1_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".a1slice")
}

pub fn video_stem(video_path: &Path) -> (PathBuf, String) {
    let dir = video_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let name = video_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let stem = name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(name)
        .to_string();
    (dir, stem)
}

pub fn transcript_json_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice.json"))
}

pub fn transcript_text_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}-transcript.txt"))
}

pub fn clips_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-clips.json"))
}

pub fn analysis_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-analysis.txt"))
}

pub fn framing_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-framing.json"))
}

pub fn captions_path(video_path: &Path) -> PathBuf {
    let (dir, stem) = video_stem(video_path);
    dir.join(format!("{stem}.a1slice-captions.json"))
}

pub fn recent_path() -> PathBuf {
    a1_dir().join("recent.json")
}

pub fn settings_path() -> PathBuf {
    a1_dir().join("settings.json")
}

pub fn models_dir() -> PathBuf {
    a1_dir().join("models")
}

pub fn model_path() -> PathBuf {
    models_dir().join("ggml-large-v3.bin")
}

pub fn preview_cache_dir() -> PathBuf {
    a1_dir().join("previews")
}

pub fn cached_preview_path(key: &str) -> PathBuf {
    preview_cache_dir().join(format!("{key}.mp4"))
}
