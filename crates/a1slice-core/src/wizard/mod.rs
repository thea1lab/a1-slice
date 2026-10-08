//! Wizard state and the one function that changes it.

mod reduce;
mod state;

pub use reduce::wizard_reduce;
pub use state::{ReturnTo, WizardAction, WizardState};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use crate::types::{
        ClipCrop, ClipSegmentWithStatus, CropRatio, LlmProvider, PipelineStage, Screen, ToolId,
        TranscriptSegment, DEFAULT_CROP,
    };
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
