//! A running export or transcription, and the agents installed on this machine.

use a1slice_core::types::{ClipSegment, TranscriptSegment};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::process::which;

pub struct Running {
    pub cancel: Arc<AtomicBool>,
    pub child: Arc<Mutex<Option<std::process::Child>>>,
    pub handle: JoinHandle<Result<JobDone, String>>,
}

#[derive(Debug, Clone)]
pub enum JobDone {
    Transcript(Vec<TranscriptSegment>),
    Clips {
        clips: Vec<ClipSegment>,
        raw: String,
    },
    CaptionLines(Vec<TranscriptSegment>),
    Folder(PathBuf),
}

pub fn list_agents() -> Vec<(&'static str, &'static str, &'static str)> {
    [
        ("grok", "Grok 4.7", "grok"),
        ("claude", "Sonnet 5.5", "claude"),
        ("sol", "Sol 5.6", "codex"),
        ("agy", "Gemini 3.8 Flash", "agy"),
    ]
    .into_iter()
    .filter(|(_, _, cmd)| which(cmd).is_some())
    .collect()
}
