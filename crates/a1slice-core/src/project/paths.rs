//! Paths for the files kept beside a video and under ~/.a1slice.

use std::path::{Path, PathBuf};

pub(super) const RECENT_LIMIT: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoStem {
    pub dir: PathBuf,
    pub stem: String,
}

impl VideoStem {
    pub fn join(&self, file_name: impl AsRef<Path>) -> PathBuf {
        if self.dir.as_os_str() == "." {
            PathBuf::from(file_name.as_ref())
        } else {
            self.dir.join(file_name)
        }
    }
}

pub fn a1slice_dir() -> PathBuf {
    home_dir().join(".a1slice")
}

fn home_dir() -> PathBuf {
    for key in ["HOME", "USERPROFILE"] {
        if let Some(home) = std::env::var_os(key) {
            if !home.is_empty() {
                return PathBuf::from(home);
            }
        }
    }
    PathBuf::from(".")
}

pub fn video_stem(video_path: &Path) -> VideoStem {
    let name = video_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = strip_one_extension(&name).to_string();
    let dir = video_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    VideoStem { dir, stem }
}

fn strip_one_extension(base: &str) -> &str {
    match base.rfind('.') {
        Some(dot) if dot + 1 < base.len() => &base[..dot],
        _ => base,
    }
}

pub fn transcript_json_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice.json", located.stem))
}

pub fn clips_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-clips.json", located.stem))
}

pub fn analysis_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-analysis.txt", located.stem))
}

pub fn framing_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-framing.json", located.stem))
}

pub fn captions_path(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}.a1slice-captions.json", located.stem))
}

pub(super) fn transcript_text_file(video_path: &Path) -> PathBuf {
    let located = video_stem(video_path);
    located.join(format!("{}-transcript.txt", located.stem))
}

pub fn transcript_text_path(video_path: &str) -> String {
    let base = video_path
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(video_path);
    let name = format!("{}-transcript.txt", strip_one_extension(base));
    match video_path.rfind(['/', '\\']) {
        Some(slash) => format!("{}{name}", &video_path[..=slash]),
        None => name,
    }
}
