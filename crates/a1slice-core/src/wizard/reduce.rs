//! Apply one wizard action and return the next state.

use std::collections::BTreeMap;

use crate::types::{
    default_model, CaptionProject, ClipCrop, ClipSegmentWithStatus, CropRatio, LlmProvider, Ms,
    PipelineStage, Screen, TranscriptSegment, VideoLanguage, DEFAULT_CROP,
};

use super::state::{
    begin_tool, keep_settings, screen_after_transcript, screen_for_tool, stored_api_key,
    with_api_key, WizardAction, WizardState,
};

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
        } => load_settings(
            state,
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
        ),
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
        } => project_loaded(
            state,
            segments,
            clips,
            raw_response,
            framing,
            captions,
            duration_ms,
        ),
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
        WizardAction::ToggleClip(id) => map_clip(state, &id, |clip| ClipSegmentWithStatus {
            approved: !clip.approved,
            ..clip.clone()
        }),
        WizardAction::UpdateClipTimes {
            id,
            start_ms,
            end_ms,
        } => map_clip(state, &id, |clip| ClipSegmentWithStatus {
            start_ms,
            end_ms,
            ..clip.clone()
        }),
        WizardAction::UpdateClipCrop { id, crop } => {
            map_clip(state, &id, |clip| ClipSegmentWithStatus {
                crop,
                ..clip.clone()
            })
        }
        WizardAction::AddClip => add_clip(state),
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

fn load_settings(
    state: &WizardState,
    provider: LlmProvider,
    model: String,
    api_key: String,
    mut api_keys: BTreeMap<String, String>,
    user_hint: String,
    language: VideoLanguage,
    entropy_thold: f64,
    max_context: i64,
    beam_size: i64,
    temperature_inc: f64,
) -> WizardState {
    if !api_key.is_empty() && !api_keys.contains_key(provider.as_str()) {
        api_keys.insert(provider.as_str().to_string(), api_key.clone());
    }
    let resolved = api_keys.get(provider.as_str()).cloned().unwrap_or(api_key);
    WizardState {
        provider,
        model,
        api_key: resolved,
        api_keys,
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

fn project_loaded(
    state: &WizardState,
    segments: Vec<TranscriptSegment>,
    clips: Vec<ClipSegmentWithStatus>,
    raw_response: String,
    framing: BTreeMap<CropRatio, ClipCrop>,
    captions: Option<CaptionProject>,
    duration_ms: Ms,
) -> WizardState {
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

fn map_clip(
    state: &WizardState,
    id: &str,
    change: impl Fn(&ClipSegmentWithStatus) -> ClipSegmentWithStatus,
) -> WizardState {
    let clips = state
        .clips
        .iter()
        .map(|clip| {
            if clip.id == id {
                change(clip)
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

fn add_clip(state: &WizardState) -> WizardState {
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
