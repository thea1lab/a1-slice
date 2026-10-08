//! Caption-edit requests, diff rows, and the messages for a bad edit.

use crate::types::TranscriptSegment;

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionEditRequest {
    pub fix_typos: bool,
    pub break_lines: bool,
    pub words_per_line: f64,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Same,
    Add,
    Remove,
    Gap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
    pub kind: DiffKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptionEditResult {
    Segments(Vec<TranscriptSegment>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionLogState {
    pub hiding_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLogState {
    pub hide_json: bool,
    pub final_text: Option<String>,
    pub pieces: Vec<String>,
}

pub(super) const NO_LINES: &str =
    "The agent did not return the edited lines. The lines were left as they are.";
pub(super) const UNREADABLE: &str =
    "The agent returned lines that could not be read. The lines were left as they are.";
pub(super) const CHANGED_TOO_MUCH: &str =
    "The agent changed too much of the file. The lines were left as they are.";
pub(super) const REMOVED_TOO_MANY: &str =
    "The agent removed too many words. The lines were left as they are.";
pub(super) const ADDED_TOO_MANY: &str =
    "The agent added too many words. The lines were left as they are.";
pub(super) const MOVED_OFF_END: &str =
    "The agent moved a line off the end of the video. The lines were left as they are.";
