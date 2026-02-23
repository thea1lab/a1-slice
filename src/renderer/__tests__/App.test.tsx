import { describe, it, expect } from 'vitest'
import { wizardReducer } from '../App'
import type {
  WizardStep,
  PipelineStage,
  LLMProvider,
  ClipSegmentWithStatus,
  TranscriptSegment
} from '../../shared/types'

function makeState(
  overrides: Partial<{
    currentStep: WizardStep
    completedSteps: WizardStep[]
    provider: LLMProvider
    model: string
    apiKey: string
    videoPath: string | null
    settingsLoaded: boolean
    transcribeStage: PipelineStage
    transcribeMessage: string
    transcribePercent: number
    transcribeError: string | null
    segments: { startMs: number; endMs: number; text: string }[]
    analyzing: boolean
    analyzePercent: number
    analyzeMessage: string
    analyzeError: string | null
    clips: ClipSegmentWithStatus[]
    rawResponse: string
    exportStage: PipelineStage
    exportMessage: string
    exportPercent: number
    exportError: string | null
    outputDir: string | null
  }> = {}
) {
  return {
    currentStep: 'select' as WizardStep,
    completedSteps: [] as WizardStep[],
    provider: 'claude' as LLMProvider,
    model: 'claude-haiku-4-5-20251001',
    apiKey: '',
    settingsLoaded: false,
    videoPath: null,
    transcribeStage: 'idle' as PipelineStage,
    transcribeMessage: '',
    transcribePercent: 0,
    transcribeError: null,
    segments: [],
    analyzing: false,
    analyzePercent: 0,
    analyzeMessage: '',
    analyzeError: null,
    clips: [],
    rawResponse: '',
    exportStage: 'idle' as PipelineStage,
    exportMessage: '',
    exportPercent: 0,
    exportError: null,
    outputDir: null,
    ...overrides
  } as ReturnType<typeof wizardReducer>
}

describe('wizardReducer', () => {
  it('sets provider and resets model to provider default', () => {
    const state = wizardReducer(makeState(), {
      type: 'SET_PROVIDER',
      provider: 'openai'
    })
    expect(state.provider).toBe('openai')
    expect(state.model).toBe('gpt-5-mini-2025-08-07')
  })

  it('sets provider back to claude with correct default model', () => {
    const state = wizardReducer(
      makeState({ provider: 'openai', model: 'gpt-5-mini-2025-08-07' }),
      { type: 'SET_PROVIDER', provider: 'claude' }
    )
    expect(state.provider).toBe('claude')
    expect(state.model).toBe('claude-haiku-4-5-20251001')
  })

  it('sets model', () => {
    const state = wizardReducer(makeState(), {
      type: 'SET_MODEL',
      model: 'claude-opus-4-20250514'
    })
    expect(state.model).toBe('claude-opus-4-20250514')
  })

  it('sets api key', () => {
    const state = wizardReducer(makeState(), {
      type: 'SET_API_KEY',
      apiKey: 'sk-test'
    })
    expect(state.apiKey).toBe('sk-test')
  })

  it('sets video path', () => {
    const state = wizardReducer(makeState(), {
      type: 'SET_VIDEO',
      videoPath: '/path/to/video.mp4'
    })
    expect(state.videoPath).toBe('/path/to/video.mp4')
  })

  it('loads settings from persistence', () => {
    const state = wizardReducer(makeState(), {
      type: 'LOAD_SETTINGS',
      provider: 'openai',
      model: 'gpt-5-mini-2025-08-07',
      apiKey: 'sk-persisted'
    })
    expect(state.provider).toBe('openai')
    expect(state.model).toBe('gpt-5-mini-2025-08-07')
    expect(state.apiKey).toBe('sk-persisted')
    expect(state.settingsLoaded).toBe(true)
  })

  it('navigates to a step via GO_TO_STEP', () => {
    const state = wizardReducer(
      makeState({ completedSteps: ['select', 'transcribe'] }),
      { type: 'GO_TO_STEP', step: 'select' }
    )
    expect(state.currentStep).toBe('select')
  })

  it('starts transcription: moves to transcribe step, marks select as completed', () => {
    const state = wizardReducer(
      makeState({ videoPath: '/video.mp4' }),
      { type: 'START_TRANSCRIBE' }
    )
    expect(state.currentStep).toBe('transcribe')
    expect(state.completedSteps).toContain('select')
    expect(state.transcribeStage).toBe('extracting')
    expect(state.transcribeError).toBeNull()
  })

  it('updates transcription progress', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'transcribe', transcribeStage: 'extracting' }),
      {
        type: 'TRANSCRIBE_PROGRESS',
        update: { stage: 'transcribing', message: 'Transcribing...', percent: 42 }
      }
    )
    expect(state.transcribeStage).toBe('transcribing')
    expect(state.transcribeMessage).toBe('Transcribing...')
    expect(state.transcribePercent).toBe(42)
  })

  it('completes transcription: moves to review-transcript, stores segments', () => {
    const segments = [{ startMs: 0, endMs: 5000, text: 'Hello' }]
    const state = wizardReducer(
      makeState({ currentStep: 'transcribe' }),
      { type: 'TRANSCRIBE_DONE', segments }
    )
    expect(state.currentStep).toBe('review-transcript')
    expect(state.completedSteps).toContain('transcribe')
    expect(state.segments).toEqual(segments)
  })

  it('handles transcription error', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'transcribe' }),
      { type: 'TRANSCRIBE_ERROR', error: 'Something broke' }
    )
    expect(state.transcribeStage).toBe('error')
    expect(state.transcribeError).toBe('Something broke')
  })

  it('starts analysis', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'review-transcript' }),
      { type: 'START_ANALYZE' }
    )
    expect(state.analyzing).toBe(true)
    expect(state.analyzeError).toBeNull()
  })

  it('completes analysis: moves to review-slices with clips', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip 1', startMs: 0, endMs: 30000, approved: true }
    ]
    const state = wizardReducer(
      makeState({ currentStep: 'review-transcript', analyzing: true }),
      { type: 'ANALYZE_DONE', clips, rawResponse: '[{"title":"Clip 1"}]' }
    )
    expect(state.currentStep).toBe('review-slices')
    expect(state.completedSteps).toContain('review-transcript')
    expect(state.analyzing).toBe(false)
    expect(state.clips).toEqual(clips)
  })

  it('handles analysis error', () => {
    const state = wizardReducer(
      makeState({ analyzing: true }),
      { type: 'ANALYZE_ERROR', error: 'API error' }
    )
    expect(state.analyzing).toBe(false)
    expect(state.analyzeError).toBe('API error')
  })

  it('toggles clip approval', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'A', startMs: 0, endMs: 1000, approved: true },
      { id: '1', title: 'B', startMs: 1000, endMs: 2000, approved: true }
    ]
    const state = wizardReducer(
      makeState({ clips }),
      { type: 'TOGGLE_CLIP', id: '0' }
    )
    expect(state.clips[0].approved).toBe(false)
    expect(state.clips[1].approved).toBe(true)
  })

  it('starts export: moves to export step, marks review-slices as completed', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'review-slices' }),
      { type: 'START_EXPORT' }
    )
    expect(state.currentStep).toBe('export')
    expect(state.completedSteps).toContain('review-slices')
    expect(state.exportStage).toBe('cutting')
  })

  it('completes export with outputDir', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'export', exportStage: 'cutting' }),
      { type: 'EXPORT_DONE', outputDir: '/output/folder' }
    )
    expect(state.exportStage).toBe('done')
    expect(state.outputDir).toBe('/output/folder')
    expect(state.completedSteps).toContain('export')
  })

  it('handles export error', () => {
    const state = wizardReducer(
      makeState({ currentStep: 'export', exportStage: 'cutting' }),
      { type: 'EXPORT_ERROR', error: 'ffmpeg failed' }
    )
    expect(state.exportStage).toBe('error')
    expect(state.exportError).toBe('ffmpeg failed')
  })

  it('resets state but preserves settings', () => {
    const state = wizardReducer(
      makeState({
        currentStep: 'export',
        completedSteps: ['select', 'transcribe', 'review-transcript', 'review-slices'],
        provider: 'openai',
        model: 'gpt-5-mini-2025-08-07',
        apiKey: 'sk-test',
        videoPath: '/video.mp4',
        outputDir: '/output',
        settingsLoaded: true,
        segments: [{ startMs: 0, endMs: 1000, text: 'hi' }]
      }),
      { type: 'RESET' }
    )
    expect(state.currentStep).toBe('select')
    expect(state.completedSteps).toEqual([])
    expect(state.provider).toBe('openai')
    expect(state.model).toBe('gpt-5-mini-2025-08-07')
    expect(state.apiKey).toBe('sk-test')
    expect(state.settingsLoaded).toBe(true)
    expect(state.videoPath).toBeNull()
    expect(state.outputDir).toBeNull()
    expect(state.segments).toEqual([])
  })

  it('stores rawResponse on ANALYZE_DONE', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip 1', startMs: 0, endMs: 30000, approved: true }
    ]
    const state = wizardReducer(
      makeState({ currentStep: 'review-transcript', analyzing: true }),
      { type: 'ANALYZE_DONE', clips, rawResponse: '{"raw":"data"}' }
    )
    expect(state.rawResponse).toBe('{"raw":"data"}')
  })

  it('updates clip times', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'A', startMs: 0, endMs: 10000, approved: true },
      { id: '1', title: 'B', startMs: 20000, endMs: 30000, approved: true }
    ]
    const state = wizardReducer(
      makeState({ clips }),
      { type: 'UPDATE_CLIP_TIMES', id: '0', startMs: 1000, endMs: 9000 }
    )
    expect(state.clips[0].startMs).toBe(1000)
    expect(state.clips[0].endMs).toBe(9000)
    expect(state.clips[1].startMs).toBe(20000)
  })

  it('loads cached transcript and jumps to review-transcript', () => {
    const segments: TranscriptSegment[] = [
      { startMs: 0, endMs: 5000, text: 'Cached' }
    ]
    const state = wizardReducer(
      makeState({ videoPath: '/video.mp4' }),
      { type: 'LOAD_CACHED_TRANSCRIPT', segments }
    )
    expect(state.currentStep).toBe('review-transcript')
    expect(state.completedSteps).toContain('select')
    expect(state.completedSteps).toContain('transcribe')
    expect(state.segments).toEqual(segments)
  })

  it('transitions through the full wizard flow', () => {
    let state = makeState()

    // Select video
    state = wizardReducer(state, { type: 'SET_VIDEO', videoPath: '/video.mp4' })
    state = wizardReducer(state, { type: 'SET_API_KEY', apiKey: 'sk-key' })

    // Start transcription
    state = wizardReducer(state, { type: 'START_TRANSCRIBE' })
    expect(state.currentStep).toBe('transcribe')

    // Transcription completes
    const segments = [{ startMs: 0, endMs: 5000, text: 'Hello world' }]
    state = wizardReducer(state, { type: 'TRANSCRIBE_DONE', segments })
    expect(state.currentStep).toBe('review-transcript')

    // Start analysis
    state = wizardReducer(state, { type: 'START_ANALYZE' })

    // Analysis completes
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip', startMs: 0, endMs: 5000, approved: true }
    ]
    state = wizardReducer(state, { type: 'ANALYZE_DONE', clips, rawResponse: '[]' })
    expect(state.currentStep).toBe('review-slices')

    // Start export
    state = wizardReducer(state, { type: 'START_EXPORT' })
    expect(state.currentStep).toBe('export')

    // Export completes
    state = wizardReducer(state, { type: 'EXPORT_DONE', outputDir: '/out' })
    expect(state.exportStage).toBe('done')
    expect(state.outputDir).toBe('/out')
  })
})
