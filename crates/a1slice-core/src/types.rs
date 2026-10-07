//! Shared values for the transcript, clips, captions, and settings.
//!
//! JSON field names match the files the Electron app already writes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Ms = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStage {
    Idle,
    Extracting,
    Downloading,
    Transcribing,
    Analyzing,
    Cutting,
    Done,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start_ms: Ms,
    pub end_ms: Ms,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CropRatio {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "16:9")]
    R16x9,
    #[serde(rename = "4:3")]
    R4x3,
    #[serde(rename = "9:16")]
    R9x16,
    #[serde(rename = "1:1")]
    Square,
}

impl CropRatio {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::R16x9 => "16:9",
            Self::R4x3 => "4:3",
            Self::R9x16 => "9:16",
            Self::Square => "1:1",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "original" => Self::Original,
            "16:9" => Self::R16x9,
            "4:3" => Self::R4x3,
            "9:16" => Self::R9x16,
            "1:1" => Self::Square,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClipCrop {
    pub ratio: CropRatio,
    /// Center of the crop window in the source frame, 0–1.
    pub cx: f64,
    pub cy: f64,
    /// 0 = largest window that fits the ratio, 1 = punched in.
    pub zoom: f64,
}

pub const DEFAULT_CROP: ClipCrop = ClipCrop {
    ratio: CropRatio::Original,
    cx: 0.5,
    cy: 0.5,
    zoom: 0.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipCategory {
    Related,
    Standalone,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSegment {
    pub title: String,
    pub start_ms: Ms,
    pub end_ms: Ms,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<ClipCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<ClipCrop>,
    /// Missing on older cache files. Treated as kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressUpdate {
    pub stage: PipelineStage,
    pub message: String,
    pub percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Claude,
    Openai,
    Opencode,
}

impl LlmProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Openai => "openai",
            Self::Opencode => "opencode",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "claude" => Self::Claude,
            "openai" => Self::Openai,
            "opencode" => Self::Opencode,
            _ => return None,
        })
    }
}

pub fn default_model(provider: LlmProvider) -> &'static str {
    match provider {
        LlmProvider::Claude => "claude-haiku-4-5",
        LlmProvider::Openai => "gpt-5-mini-2025-08-07",
        LlmProvider::Opencode => "minimax-m2.7",
    }
}

/// OpenCode Zen gateway (https://opencode.ai/zen).
pub const OPENCODE_ZEN_BASE_URL: &str = "https://opencode.ai/zen/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoLanguage {
    Auto,
    En,
    Pt,
    Es,
}

impl VideoLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::En => "en",
            Self::Pt => "pt",
            Self::Es => "es",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "auto" => Self::Auto,
            "en" => Self::En,
            "pt" => Self::Pt,
            "es" => Self::Es,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    /// Keys stored per provider so switching providers can restore them.
    pub api_keys: BTreeMap<String, String>,
    pub user_hint: String,
    pub language: VideoLanguage,
    pub entropy_thold: f64,
    pub max_context: i64,
    pub beam_size: i64,
    pub temperature_inc: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Claude,
            model: default_model(LlmProvider::Claude).to_string(),
            api_key: String::new(),
            api_keys: BTreeMap::new(),
            user_hint: String::new(),
            language: VideoLanguage::Auto,
            entropy_thold: 2.8,
            max_context: 64,
            beam_size: 5,
            temperature_inc: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolId {
    Transcribe,
    Find,
    Reframe,
    Captions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    #[serde(rename = "home")]
    Home,
    #[serde(rename = "transcribe")]
    Transcribe,
    #[serde(rename = "transcribe-done")]
    TranscribeDone,
    #[serde(rename = "find")]
    Find,
    #[serde(rename = "review")]
    Review,
    #[serde(rename = "export")]
    Export,
    #[serde(rename = "reframe")]
    Reframe,
    #[serde(rename = "captions")]
    Captions,
    #[serde(rename = "fix-words")]
    FixWords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleExport {
    Off,
    Srt,
    Burn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionLook {
    Srt,
    Burn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionSource {
    Transcript,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionColor {
    White,
    Cream,
    Yellow,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionPosition {
    Bottom,
    Middle,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionFont {
    Sans,
    Serif,
    Mono,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionStyle {
    pub color: CaptionColor,
    /// `#rrggbb` when the colour is not one of the named presets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_color: Option<String>,
    pub position: CaptionPosition,
    pub size: CaptionSize,
    /// ASS script pixels at PlayResY 288. Overrides `size` when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_size: Option<i64>,
    pub font: CaptionFont,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionProject {
    pub source: CaptionSource,
    pub look: CaptionLook,
    pub style: CaptionStyle,
    pub cues: Vec<TranscriptSegment>,
    /// Caption file chosen in place of this video's transcript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionFilePick {
    pub path: String,
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInspection {
    pub has_transcript: bool,
    pub segment_count: usize,
    pub has_clips: bool,
    pub clip_count: usize,
    pub has_framing: bool,
    pub has_captions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentVideo {
    pub path: String,
    pub inspection: VideoInspection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectData {
    pub segments: Vec<TranscriptSegment>,
    pub clips: Vec<ClipSegment>,
    pub raw_response: String,
    pub framing: BTreeMap<CropRatio, ClipCrop>,
    pub captions: Option<CaptionProject>,
    pub duration_ms: Ms,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClipSegmentWithStatus {
    pub id: String,
    pub title: String,
    pub start_ms: Ms,
    pub end_ms: Ms,
    pub category: Option<ClipCategory>,
    pub topic: Option<String>,
    pub crop: ClipCrop,
    pub approved: bool,
}

impl ClipSegmentWithStatus {
    pub fn to_stored(&self) -> ClipSegment {
        ClipSegment {
            title: self.title.clone(),
            start_ms: self.start_ms,
            end_ms: self.end_ms,
            category: self.category,
            topic: self.topic.clone(),
            crop: Some(self.crop),
            approved: Some(self.approved),
        }
    }
}
