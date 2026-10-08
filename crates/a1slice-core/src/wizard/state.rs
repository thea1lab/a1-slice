//! Wizard state, the actions that change it, and the tool transitions.

use crate::types::{
    default_model, CaptionProject, ClipCrop, ClipSegmentWithStatus, CropRatio, LlmProvider, Ms,
    PipelineStage, ProgressUpdate, Screen, SubtitleExport, ToolId, TranscriptSegment,
    VideoLanguage,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct WizardState {
    pub screen: Screen,
    pub return_to: Option<ReturnTo>,
    pub project_ready: bool,
    pub provider: LlmProvider,
    pub model: String,
    pub api_key: String,
    pub api_keys: BTreeMap<String, String>,
    pub user_hint: String,
    pub language: VideoLanguage,
    pub entropy_thold: f64,
    pub max_context: i64,
    pub beam_size: i64,
    pub temperature_inc: f64,
    pub settings_loaded: bool,
    pub video_path: Option<String>,
    pub video_duration_ms: Ms,
    pub transcribe_stage: PipelineStage,
    pub transcribe_message: String,
    pub transcribe_percent: f64,
    pub transcribe_error: Option<String>,
    pub segments: Vec<TranscriptSegment>,
    pub analyzing: bool,
    pub analyze_percent: f64,
    pub analyze_message: String,
    pub analyze_error: Option<String>,
    pub clips: Vec<ClipSegmentWithStatus>,
    pub raw_response: String,
    pub framing: BTreeMap<CropRatio, ClipCrop>,
    pub captions: Option<CaptionProject>,
    pub export_subtitles: SubtitleExport,
    pub export_stage: PipelineStage,
    pub export_message: String,
    pub export_percent: f64,
    pub export_error: Option<String>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnTo {
    Find,
    Captions,
}

impl Default for WizardState {
    fn default() -> Self {
        Self {
            screen: Screen::Home,
            return_to: None,
            project_ready: false,
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
            settings_loaded: false,
            video_path: None,
            video_duration_ms: 0,
            transcribe_stage: PipelineStage::Idle,
            transcribe_message: String::new(),
            transcribe_percent: 0.0,
            transcribe_error: None,
            segments: Vec::new(),
            analyzing: false,
            analyze_percent: 0.0,
            analyze_message: String::new(),
            analyze_error: None,
            clips: Vec::new(),
            raw_response: String::new(),
            framing: BTreeMap::new(),
            captions: None,
            export_subtitles: SubtitleExport::Srt,
            export_stage: PipelineStage::Idle,
            export_message: String::new(),
            export_percent: 0.0,
            export_error: None,
            output_dir: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum WizardAction {
    LoadSettings {
        provider: LlmProvider,
        model: String,
        api_key: String,
        api_keys: BTreeMap<String, String>,
        user_hint: String,
        language: VideoLanguage,
        entropy_thold: f64,
        max_context: i64,
        beam_size: i64,
        temperature_inc: f64,
    },
    SetProvider(LlmProvider),
    SetModel(String),
    SetApiKey(String),
    SetUserHint(String),
    SetLanguage(VideoLanguage),
    SetEntropyThold(f64),
    SetMaxContext(i64),
    SetBeamSize(i64),
    SetTemperatureInc(f64),
    SetVideo(String),
    OpenTool {
        tool: ToolId,
        video_path: String,
    },
    EnterTool(ToolId),
    ProjectLoaded {
        segments: Vec<TranscriptSegment>,
        clips: Vec<ClipSegmentWithStatus>,
        raw_response: String,
        framing: BTreeMap<CropRatio, ClipCrop>,
        captions: Option<CaptionProject>,
        duration_ms: Ms,
    },
    ShowScreen(Screen),
    PrepareTranscribe(ReturnTo),
    SetReturnTo(Option<ReturnTo>),
    GoHome,
    StartTranscribe,
    TranscribeProgress(ProgressUpdate),
    TranscribeDone(Vec<TranscriptSegment>),
    TranscribeError(String),
    UseSavedTranscript,
    StartAnalyze,
    AnalyzeProgress(ProgressUpdate),
    AnalyzeDone {
        clips: Vec<ClipSegmentWithStatus>,
        raw_response: String,
    },
    AnalyzeError(String),
    ToggleClip(String),
    UpdateClipTimes {
        id: String,
        start_ms: Ms,
        end_ms: Ms,
    },
    UpdateClipCrop {
        id: String,
        crop: ClipCrop,
    },
    AddClip,
    SetFraming(BTreeMap<CropRatio, ClipCrop>),
    SetCaptions(CaptionProject),
    SetSegments(Vec<TranscriptSegment>),
    SetExportSubtitles(SubtitleExport),
    StartExport,
    StartRender,
    ExportProgress(ProgressUpdate),
    ExportDone(String),
    ExportError(String),
}

pub(super) fn stored_api_key(api_keys: &BTreeMap<String, String>, provider: LlmProvider) -> String {
    api_keys.get(provider.as_str()).cloned().unwrap_or_default()
}

pub(super) fn with_api_key(
    api_keys: &BTreeMap<String, String>,
    provider: LlmProvider,
    api_key: &str,
) -> BTreeMap<String, String> {
    let mut next = api_keys.clone();
    if api_key.is_empty() {
        next.remove(provider.as_str());
    } else {
        next.insert(provider.as_str().to_string(), api_key.to_string());
    }
    next
}

pub(super) fn screen_for_tool(tool: ToolId) -> Screen {
    match tool {
        ToolId::Transcribe => Screen::Transcribe,
        ToolId::Find => Screen::Find,
        ToolId::Reframe => Screen::Reframe,
        ToolId::Captions => Screen::Captions,
    }
}

pub(super) fn begin_tool(
    state: &WizardState,
    screen: Screen,
    video_path: Option<String>,
) -> WizardState {
    WizardState {
        screen,
        video_path,
        return_to: None,
        project_ready: false,
        segments: Vec::new(),
        clips: Vec::new(),
        raw_response: String::new(),
        framing: BTreeMap::new(),
        captions: None,
        video_duration_ms: 0,
        transcribe_stage: PipelineStage::Idle,
        transcribe_message: String::new(),
        transcribe_percent: 0.0,
        transcribe_error: None,
        analyzing: false,
        analyze_error: None,
        analyze_percent: 0.0,
        analyze_message: String::new(),
        export_stage: PipelineStage::Idle,
        export_message: String::new(),
        export_percent: 0.0,
        export_error: None,
        output_dir: None,
        ..state.clone()
    }
}

pub(super) fn screen_after_transcript(state: &WizardState) -> Screen {
    match state.return_to {
        Some(ReturnTo::Find) => Screen::Find,
        Some(ReturnTo::Captions) => Screen::Captions,
        None => Screen::TranscribeDone,
    }
}

pub(super) fn keep_settings(state: &WizardState) -> WizardState {
    WizardState {
        provider: state.provider,
        model: state.model.clone(),
        api_key: state.api_key.clone(),
        api_keys: state.api_keys.clone(),
        user_hint: state.user_hint.clone(),
        language: state.language,
        entropy_thold: state.entropy_thold,
        max_context: state.max_context,
        beam_size: state.beam_size,
        temperature_inc: state.temperature_inc,
        settings_loaded: state.settings_loaded,
        ..WizardState::default()
    }
}
