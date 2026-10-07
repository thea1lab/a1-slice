use std::collections::BTreeMap;

use crate::types::{
    default_model, CaptionProject, ClipCrop, ClipSegmentWithStatus, CropRatio, LlmProvider, Ms,
    PipelineStage, ProgressUpdate, Screen, SubtitleExport, ToolId, TranscriptSegment,
    VideoLanguage, DEFAULT_CROP,
};

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

fn stored_api_key(api_keys: &BTreeMap<String, String>, provider: LlmProvider) -> String {
    api_keys.get(provider.as_str()).cloned().unwrap_or_default()
}

fn with_api_key(
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

fn screen_for_tool(tool: ToolId) -> Screen {
    match tool {
        ToolId::Transcribe => Screen::Transcribe,
        ToolId::Find => Screen::Find,
        ToolId::Reframe => Screen::Reframe,
        ToolId::Captions => Screen::Captions,
    }
}

fn begin_tool(state: &WizardState, screen: Screen, video_path: Option<String>) -> WizardState {
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

fn screen_after_transcript(state: &WizardState) -> Screen {
    match state.return_to {
        Some(ReturnTo::Find) => Screen::Find,
        Some(ReturnTo::Captions) => Screen::Captions,
        None => Screen::TranscribeDone,
    }
}

fn keep_settings(state: &WizardState) -> WizardState {
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

pub fn wizard_reduce(state: &WizardState, action: WizardAction) -> WizardState {
    match action {
        WizardAction::LoadSettings {
            provider,
            model,
            api_key,
            api_keys,
            user_hint,
            language,
            entropy_thold,
            max_context,
            beam_size,
            temperature_inc,
        } => {
            let mut keys = api_keys;
            if !api_key.is_empty() && !keys.contains_key(provider.as_str()) {
                keys.insert(provider.as_str().to_string(), api_key.clone());
            }
            let resolved = keys.get(provider.as_str()).cloned().unwrap_or(api_key);
            WizardState {
                provider,
                model,
                api_key: resolved,
                api_keys: keys,
                user_hint,
                language,
                entropy_thold,
                max_context,
                beam_size,
                temperature_inc,
                settings_loaded: true,
                ..state.clone()
            }
        }
        WizardAction::SetProvider(provider) => {
            if provider == state.provider {
                return state.clone();
            }
            WizardState {
                provider,
                model: default_model(provider).to_string(),
                api_key: stored_api_key(&state.api_keys, provider),
                ..state.clone()
            }
        }
        WizardAction::SetModel(model) => WizardState {
            model,
            ..state.clone()
        },
        WizardAction::SetApiKey(api_key) => WizardState {
            api_keys: with_api_key(&state.api_keys, state.provider, &api_key),
            api_key,
            ..state.clone()
        },
        WizardAction::SetUserHint(user_hint) => WizardState {
            user_hint,
            ..state.clone()
        },
        WizardAction::SetLanguage(language) => WizardState {
            language,
            ..state.clone()
        },
        WizardAction::SetEntropyThold(entropy_thold) => WizardState {
            entropy_thold,
            ..state.clone()
        },
        WizardAction::SetMaxContext(max_context) => WizardState {
            max_context,
            ..state.clone()
        },
        WizardAction::SetBeamSize(beam_size) => WizardState {
            beam_size,
            ..state.clone()
        },
        WizardAction::SetTemperatureInc(temperature_inc) => WizardState {
            temperature_inc,
            ..state.clone()
        },
        WizardAction::SetVideo(video_path) => WizardState {
            video_path: Some(video_path),
            ..state.clone()
        },
        WizardAction::EnterTool(tool) => begin_tool(state, screen_for_tool(tool), None),
        WizardAction::OpenTool { tool, video_path } => {
            begin_tool(state, screen_for_tool(tool), Some(video_path))
        }
        WizardAction::ProjectLoaded {
            segments,
            clips,
            raw_response,
            framing,
            captions,
            duration_ms,
        } => {
            let screen = if state.screen == Screen::Find && !clips.is_empty() {
                Screen::Review
            } else {
                state.screen
            };
            WizardState {
                project_ready: true,
                segments,
                clips,
                raw_response,
                framing,
                captions,
                video_duration_ms: duration_ms,
                screen,
                ..state.clone()
            }
        }
        WizardAction::ShowScreen(screen) => WizardState {
            screen,
            ..state.clone()
        },
        WizardAction::PrepareTranscribe(return_to) => WizardState {
            screen: Screen::Transcribe,
            return_to: Some(return_to),
            transcribe_stage: PipelineStage::Idle,
            transcribe_error: None,
            transcribe_message: String::new(),
            transcribe_percent: 0.0,
            ..state.clone()
        },
        WizardAction::SetReturnTo(return_to) => WizardState {
            return_to,
            ..state.clone()
        },
        WizardAction::GoHome => keep_settings(state),
        WizardAction::StartTranscribe => WizardState {
            screen: Screen::Transcribe,
            transcribe_stage: PipelineStage::Extracting,
            transcribe_message: String::new(),
            transcribe_percent: 0.0,
            transcribe_error: None,
            ..state.clone()
        },
        WizardAction::TranscribeProgress(update) => WizardState {
            transcribe_stage: update.stage,
            transcribe_message: update.message,
            transcribe_percent: update.percent,
            ..state.clone()
        },
        WizardAction::TranscribeDone(segments) => WizardState {
            screen: screen_after_transcript(state),
            return_to: None,
            transcribe_stage: PipelineStage::Done,
            transcribe_percent: 100.0,
            segments,
            project_ready: true,
            ..state.clone()
        },
        WizardAction::TranscribeError(error) => WizardState {
            transcribe_stage: PipelineStage::Error,
            transcribe_error: Some(error),
            ..state.clone()
        },
        WizardAction::UseSavedTranscript => WizardState {
            screen: screen_after_transcript(state),
            return_to: None,
            transcribe_stage: PipelineStage::Done,
            transcribe_percent: 100.0,
            ..state.clone()
        },
        WizardAction::StartAnalyze => WizardState {
            analyzing: true,
            analyze_percent: 0.0,
            analyze_message: "Reading the transcript and marking clips…".into(),
            analyze_error: None,
            ..state.clone()
        },
        WizardAction::AnalyzeProgress(update) => WizardState {
            analyze_percent: update.percent,
            analyze_message: update.message,
            ..state.clone()
        },
        WizardAction::AnalyzeDone {
            clips,
            raw_response,
        } => WizardState {
            screen: Screen::Review,
            analyzing: false,
            analyze_percent: 100.0,
            clips,
            raw_response,
            ..state.clone()
        },
        WizardAction::AnalyzeError(error) => WizardState {
            analyzing: false,
            analyze_error: Some(error),
            ..state.clone()
        },
        WizardAction::ToggleClip(id) => {
            let clips = state
                .clips
                .iter()
                .map(|clip| {
                    if clip.id == id {
                        ClipSegmentWithStatus {
                            approved: !clip.approved,
                            ..clip.clone()
                        }
                    } else {
                        clip.clone()
                    }
                })
                .collect();
            WizardState {
                clips,
                ..state.clone()
            }
        }
        WizardAction::UpdateClipTimes {
            id,
            start_ms,
            end_ms,
        } => {
            let clips = state
                .clips
                .iter()
                .map(|clip| {
                    if clip.id == id {
                        ClipSegmentWithStatus {
                            start_ms,
                            end_ms,
                            ..clip.clone()
                        }
                    } else {
                        clip.clone()
                    }
                })
                .collect();
            WizardState {
                clips,
                ..state.clone()
            }
        }
        WizardAction::UpdateClipCrop { id, crop } => {
            let clips = state
                .clips
                .iter()
                .map(|clip| {
                    if clip.id == id {
                        ClipSegmentWithStatus {
                            crop,
                            ..clip.clone()
                        }
                    } else {
                        clip.clone()
                    }
                })
                .collect();
            WizardState {
                clips,
                ..state.clone()
            }
        }
        WizardAction::AddClip => {
            let duration = if state.video_duration_ms > 0 {
                state.video_duration_ms
            } else {
                60_000
            };
            let end_ms = 1000.max(duration.min(30_000));
            let clip = ClipSegmentWithStatus {
                id: format!("manual-{}", state.clips.len()),
                title: format!("Clip {}", state.clips.len() + 1),
                start_ms: 0,
                end_ms,
                category: None,
                topic: None,
                approved: true,
                crop: DEFAULT_CROP,
            };
            let mut clips = state.clips.clone();
            clips.push(clip);
            WizardState {
                screen: Screen::Review,
                clips,
                ..state.clone()
            }
        }
        WizardAction::SetFraming(framing) => WizardState {
            framing,
            ..state.clone()
        },
        WizardAction::SetCaptions(captions) => WizardState {
            captions: Some(captions),
            ..state.clone()
        },
        WizardAction::SetSegments(segments) => WizardState {
            segments,
            ..state.clone()
        },
        WizardAction::SetExportSubtitles(export_subtitles) => WizardState {
            export_subtitles,
            ..state.clone()
        },
        WizardAction::StartExport => WizardState {
            screen: Screen::Export,
            export_stage: PipelineStage::Cutting,
            export_message: String::new(),
            export_percent: 0.0,
            export_error: None,
            output_dir: None,
            ..state.clone()
        },
        WizardAction::StartRender => WizardState {
            export_stage: PipelineStage::Cutting,
            export_message: String::new(),
            export_percent: 0.0,
            export_error: None,
            output_dir: None,
            ..state.clone()
        },
        WizardAction::ExportProgress(update) => WizardState {
            export_stage: update.stage,
            export_message: update.message,
            export_percent: update.percent,
            ..state.clone()
        },
        WizardAction::ExportDone(output_dir) => WizardState {
            export_stage: PipelineStage::Done,
            export_percent: 100.0,
            output_dir: Some(output_dir),
            ..state.clone()
        },
        WizardAction::ExportError(error) => WizardState {
            export_stage: PipelineStage::Error,
            export_error: Some(error),
            ..state.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CropRatio;

    fn clip(id: &str, title: &str, approved: bool) -> ClipSegmentWithStatus {
        ClipSegmentWithStatus {
            id: id.into(),
            title: title.into(),
            start_ms: 0,
            end_ms: 1000,
            category: None,
            topic: None,
            crop: DEFAULT_CROP,
            approved,
        }
    }

    #[test]
    fn provider_resets_the_model_and_restores_the_key() {
        let state = wizard_reduce(
            &WizardState::default(),
            WizardAction::SetProvider(LlmProvider::Openai),
        );
        assert_eq!(state.provider, LlmProvider::Openai);
        assert_eq!(state.model, "gpt-5-mini-2025-08-07");
        let state = wizard_reduce(&state, WizardAction::SetProvider(LlmProvider::Claude));
        assert_eq!(state.model, "claude-haiku-4-5");
        let state = wizard_reduce(
            &WizardState::default(),
            WizardAction::SetProvider(LlmProvider::Opencode),
        );
        assert_eq!(state.model, "minimax-m2.7");
    }

    #[test]
    fn api_key_is_stored_per_provider() {
        let state = wizard_reduce(
            &WizardState::default(),
            WizardAction::SetApiKey("sk-ant".into()),
        );
        assert_eq!(state.api_key, "sk-ant");
        assert_eq!(
            state.api_keys.get("claude").map(String::as_str),
            Some("sk-ant")
        );
        let state = wizard_reduce(&state, WizardAction::SetProvider(LlmProvider::Opencode));
        assert_eq!(state.api_key, "");
        let state = wizard_reduce(&state, WizardAction::SetProvider(LlmProvider::Claude));
        assert_eq!(state.api_key, "sk-ant");
    }

    #[test]
    fn open_and_enter_tool() {
        let mut prior = WizardState::default();
        prior.video_path = Some("/old.mp4".into());
        prior.segments = vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "old".into(),
        }];
        let state = wizard_reduce(&prior, WizardAction::EnterTool(ToolId::Reframe));
        assert_eq!(state.screen, Screen::Reframe);
        assert!(state.video_path.is_none());
        assert!(state.segments.is_empty());

        let state = wizard_reduce(
            &prior,
            WizardAction::OpenTool {
                tool: ToolId::Transcribe,
                video_path: "/video.mp4".into(),
            },
        );
        assert_eq!(state.screen, Screen::Transcribe);
        assert_eq!(state.video_path.as_deref(), Some("/video.mp4"));
        assert!(!state.project_ready);
    }

    #[test]
    fn transcribe_flow_returns_to_find() {
        let state = wizard_reduce(&WizardState::default(), WizardAction::StartTranscribe);
        assert_eq!(state.transcribe_stage, PipelineStage::Extracting);
        let mut state = wizard_reduce(&state, WizardAction::PrepareTranscribe(ReturnTo::Find));
        state.segments = vec![TranscriptSegment {
            start_ms: 0,
            end_ms: 1000,
            text: "Hi".into(),
        }];
        let done = wizard_reduce(
            &state,
            WizardAction::TranscribeDone(vec![TranscriptSegment {
                start_ms: 0,
                end_ms: 1000,
                text: "Hi".into(),
            }]),
        );
        assert_eq!(done.screen, Screen::Find);
        assert!(done.return_to.is_none());
        let err = wizard_reduce(
            &WizardState::default(),
            WizardAction::TranscribeError("Something broke".into()),
        );
        assert_eq!(err.transcribe_error.as_deref(), Some("Something broke"));
    }

    #[test]
    fn find_review_export() {
        let mut state = WizardState {
            screen: Screen::Find,
            ..WizardState::default()
        };
        state = wizard_reduce(
            &state,
            WizardAction::ProjectLoaded {
                segments: vec![TranscriptSegment {
                    start_ms: 0,
                    end_ms: 1000,
                    text: "Hi".into(),
                }],
                clips: vec![],
                raw_response: String::new(),
                framing: BTreeMap::new(),
                captions: None,
                duration_ms: 4000,
            },
        );
        assert_eq!(state.screen, Screen::Find);
        assert!(state.project_ready);
        let clips = vec![ClipSegmentWithStatus {
            approved: false,
            ..clip("0", "Clip", true)
        }];
        state = wizard_reduce(
            &state,
            WizardAction::ProjectLoaded {
                segments: vec![],
                clips,
                raw_response: String::new(),
                framing: BTreeMap::new(),
                captions: None,
                duration_ms: 0,
            },
        );
        assert_eq!(state.screen, Screen::Review);
        assert!(!state.clips[0].approved);
        state = wizard_reduce(&state, WizardAction::ToggleClip("0".into()));
        assert!(state.clips[0].approved);
        state = wizard_reduce(
            &state,
            WizardAction::UpdateClipCrop {
                id: "0".into(),
                crop: ClipCrop {
                    ratio: CropRatio::R9x16,
                    cx: 0.4,
                    cy: 0.5,
                    zoom: 0.2,
                },
            },
        );
        assert_eq!(state.clips[0].crop.ratio, CropRatio::R9x16);
        state = wizard_reduce(
            &WizardState {
                video_duration_ms: 120_000,
                ..WizardState::default()
            },
            WizardAction::AddClip,
        );
        assert_eq!(state.clips[0].end_ms, 30_000);
        state = wizard_reduce(&state, WizardAction::StartExport);
        assert_eq!(state.screen, Screen::Export);
        state = wizard_reduce(&state, WizardAction::ExportDone("/output/folder".into()));
        assert_eq!(state.output_dir.as_deref(), Some("/output/folder"));
        state = wizard_reduce(&state, WizardAction::ExportError("ffmpeg failed".into()));
        assert_eq!(state.export_error.as_deref(), Some("ffmpeg failed"));
    }

    #[test]
    fn home_keeps_settings() {
        let mut state = WizardState::default();
        state.screen = Screen::Export;
        state.provider = LlmProvider::Openai;
        state.api_key = "sk-test".into();
        state.settings_loaded = true;
        state.video_path = Some("/video.mp4".into());
        state.output_dir = Some("/output".into());
        let state = wizard_reduce(&state, WizardAction::GoHome);
        assert_eq!(state.screen, Screen::Home);
        assert_eq!(state.provider, LlmProvider::Openai);
        assert_eq!(state.api_key, "sk-test");
        assert!(state.settings_loaded);
        assert!(state.video_path.is_none());
        assert!(state.output_dir.is_none());
    }
}
