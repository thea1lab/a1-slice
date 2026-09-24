import { describe, it, expect } from 'vitest'
import { initialWizardState, wizardReducer, type WizardState } from '../state'
import type { ClipSegmentWithStatus, TranscriptSegment } from '../../shared/types'

function makeState(overrides: Partial<WizardState> = {}): WizardState {
  return { ...initialWizardState, ...overrides }
}

describe('wizardReducer', () => {
  it('sets provider and resets model to provider default', () => {
    const state = wizardReducer(makeState(), { type: 'SET_PROVIDER', provider: 'openai' })
    expect(state.provider).toBe('openai')
    expect(state.model).toBe('gpt-5-mini-2025-08-07')
  })

  it('sets provider back to claude with correct default model', () => {
    const state = wizardReducer(
      makeState({ provider: 'openai', model: 'gpt-5-mini-2025-08-07' }),
      { type: 'SET_PROVIDER', provider: 'claude' }
    )
    expect(state.provider).toBe('claude')
    expect(state.model).toBe('claude-haiku-4-5')
  })

  it('sets provider to opencode with default OpenCode model', () => {
    const state = wizardReducer(makeState(), { type: 'SET_PROVIDER', provider: 'opencode' })
    expect(state.provider).toBe('opencode')
    expect(state.model).toBe('minimax-m2.7')
  })

  it('sets model', () => {
    const state = wizardReducer(makeState(), { type: 'SET_MODEL', model: 'claude-opus-4-20250514' })
    expect(state.model).toBe('claude-opus-4-20250514')
  })

  it('sets api key', () => {
    const state = wizardReducer(makeState(), { type: 'SET_API_KEY', apiKey: 'sk-test' })
    expect(state.apiKey).toBe('sk-test')
    expect(state.apiKeys.claude).toBe('sk-test')
  })

  it('keeps the api key when only the model changes', () => {
    let state = wizardReducer(makeState(), { type: 'SET_API_KEY', apiKey: 'sk-haiku' })
    state = wizardReducer(state, { type: 'SET_MODEL', model: 'claude-opus-4' })
    expect(state.model).toBe('claude-opus-4')
    expect(state.apiKey).toBe('sk-haiku')
  })

  it('clears the api key when switching providers and restores it when switching back', () => {
    let state = wizardReducer(makeState(), { type: 'SET_API_KEY', apiKey: 'sk-ant' })
    state = wizardReducer(state, { type: 'SET_PROVIDER', provider: 'opencode' })
    expect(state.apiKey).toBe('')
    state = wizardReducer(state, { type: 'SET_PROVIDER', provider: 'claude' })
    expect(state.apiKey).toBe('sk-ant')
  })

  it('sets video path', () => {
    const state = wizardReducer(makeState(), { type: 'SET_VIDEO', videoPath: '/path/to/video.mp4' })
    expect(state.videoPath).toBe('/path/to/video.mp4')
  })

  it('loads settings from persistence', () => {
    const state = wizardReducer(makeState(), {
      type: 'LOAD_SETTINGS',
      provider: 'openai',
      model: 'gpt-5-mini-2025-08-07',
      apiKey: 'sk-persisted',
      userHint: 'focus on demos',
      language: 'auto',
      entropyThold: 2.8,
      maxContext: 64,
      beamSize: 5,
      temperatureInc: 0.1
    })
    expect(state.provider).toBe('openai')
    expect(state.apiKey).toBe('sk-persisted')
    expect(state.userHint).toBe('focus on demos')
    expect(state.settingsLoaded).toBe(true)
  })

  it('enters a tool without a video so the file picker stays closed', () => {
    const state = wizardReducer(
      makeState({
        videoPath: '/old.mp4',
        segments: [{ startMs: 0, endMs: 1000, text: 'old' }]
      }),
      { type: 'ENTER_TOOL', tool: 'reframe' }
    )
    expect(state.screen).toBe('reframe')
    expect(state.videoPath).toBeNull()
    expect(state.segments).toEqual([])
  })

  it('opens a tool on a video and clears the previous project', () => {
    const state = wizardReducer(
      makeState({
        clips: [{ id: '0', title: 'Old', startMs: 0, endMs: 1000, approved: true }],
        segments: [{ startMs: 0, endMs: 1000, text: 'old' }]
      }),
      { type: 'OPEN_TOOL', tool: 'transcribe', videoPath: '/video.mp4' }
    )
    expect(state.screen).toBe('transcribe')
    expect(state.videoPath).toBe('/video.mp4')
    expect(state.clips).toEqual([])
    expect(state.segments).toEqual([])
    expect(state.projectReady).toBe(false)
  })

  it('starts transcription on the transcribe screen', () => {
    const state = wizardReducer(makeState({ videoPath: '/video.mp4' }), { type: 'START_TRANSCRIBE' })
    expect(state.screen).toBe('transcribe')
    expect(state.transcribeStage).toBe('extracting')
    expect(state.transcribeError).toBeNull()
  })

  it('updates transcription progress', () => {
    const state = wizardReducer(makeState({ screen: 'transcribe', transcribeStage: 'extracting' }), {
      type: 'TRANSCRIBE_PROGRESS',
      update: { stage: 'transcribing', message: 'Transcribing...', percent: 42 }
    })
    expect(state.transcribeStage).toBe('transcribing')
    expect(state.transcribePercent).toBe(42)
  })

  it('finishes transcription on its own screen', () => {
    const segments = [{ startMs: 0, endMs: 5000, text: 'Hello' }]
    const state = wizardReducer(makeState({ screen: 'transcribe' }), {
      type: 'TRANSCRIBE_DONE',
      segments
    })
    expect(state.screen).toBe('transcribe-done')
    expect(state.segments).toEqual(segments)
  })

  it('opens transcribe from another tool without wiping the transcript', () => {
    const segments = [{ startMs: 0, endMs: 1000, text: 'Hi' }]
    const state = wizardReducer(
      makeState({ screen: 'find', segments, transcribeStage: 'done', videoPath: '/v.mp4' }),
      { type: 'PREPARE_TRANSCRIBE', returnTo: 'find' }
    )
    expect(state.screen).toBe('transcribe')
    expect(state.returnTo).toBe('find')
    expect(state.transcribeStage).toBe('idle')
    expect(state.segments).toEqual(segments)
  })

  it('returns to find best parts when transcription was started from there', () => {
    const state = wizardReducer(makeState({ screen: 'transcribe', returnTo: 'find' }), {
      type: 'TRANSCRIBE_DONE',
      segments: [{ startMs: 0, endMs: 1000, text: 'Hi' }]
    })
    expect(state.screen).toBe('find')
    expect(state.returnTo).toBeNull()
  })

  it('handles transcription error', () => {
    const state = wizardReducer(makeState({ screen: 'transcribe' }), {
      type: 'TRANSCRIBE_ERROR',
      error: 'Something broke'
    })
    expect(state.transcribeStage).toBe('error')
    expect(state.transcribeError).toBe('Something broke')
  })

  it('stays on find when the loaded video has no clips', () => {
    const state = wizardReducer(makeState({ screen: 'find' }), {
      type: 'PROJECT_LOADED',
      segments: [{ startMs: 0, endMs: 1000, text: 'Hi' }],
      clips: [],
      rawResponse: '',
      framing: {},
      captions: null,
      durationMs: 4000
    })
    expect(state.screen).toBe('find')
    expect(state.projectReady).toBe(true)
    expect(state.videoDurationMs).toBe(4000)
  })

  it('opens review when find loads saved clips', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip', startMs: 0, endMs: 1000, approved: false }
    ]
    const state = wizardReducer(makeState({ screen: 'find' }), {
      type: 'PROJECT_LOADED',
      segments: [],
      clips,
      rawResponse: '',
      framing: {},
      captions: null,
      durationMs: 0
    })
    expect(state.screen).toBe('review')
    expect(state.clips[0].approved).toBe(false)
  })

  it('starts analysis', () => {
    const state = wizardReducer(makeState({ screen: 'find' }), { type: 'START_ANALYZE' })
    expect(state.analyzing).toBe(true)
    expect(state.analyzeError).toBeNull()
  })

  it('completes analysis on the review screen', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip 1', startMs: 0, endMs: 30000, approved: true }
    ]
    const state = wizardReducer(makeState({ screen: 'find', analyzing: true }), {
      type: 'ANALYZE_DONE',
      clips,
      rawResponse: '[{"title":"Clip 1"}]'
    })
    expect(state.screen).toBe('review')
    expect(state.analyzing).toBe(false)
    expect(state.clips).toEqual(clips)
    expect(state.rawResponse).toBe('[{"title":"Clip 1"}]')
  })

  it('handles analysis error', () => {
    const state = wizardReducer(makeState({ analyzing: true }), {
      type: 'ANALYZE_ERROR',
      error: 'API error'
    })
    expect(state.analyzing).toBe(false)
    expect(state.analyzeError).toBe('API error')
  })

  it('toggles clip approval', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'A', startMs: 0, endMs: 1000, approved: true },
      { id: '1', title: 'B', startMs: 1000, endMs: 2000, approved: true }
    ]
    const state = wizardReducer(makeState({ clips }), { type: 'TOGGLE_CLIP', id: '0' })
    expect(state.clips[0].approved).toBe(false)
    expect(state.clips[1].approved).toBe(true)
  })

  it('updates clip crop and times', () => {
    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'A', startMs: 0, endMs: 10000, approved: true }
    ]
    const crop = { ratio: '9:16' as const, cx: 0.4, cy: 0.5, zoom: 0.2 }
    let state = wizardReducer(makeState({ clips }), { type: 'UPDATE_CLIP_CROP', id: '0', crop })
    state = wizardReducer(state, { type: 'UPDATE_CLIP_TIMES', id: '0', startMs: 1000, endMs: 9000 })
    expect(state.clips[0].crop).toEqual(crop)
    expect(state.clips[0].startMs).toBe(1000)
    expect(state.clips[0].endMs).toBe(9000)
  })

  it('adds a manual range and opens review', () => {
    const state = wizardReducer(makeState({ videoDurationMs: 120000 }), { type: 'ADD_CLIP' })
    expect(state.screen).toBe('review')
    expect(state.clips).toHaveLength(1)
    expect(state.clips[0].endMs).toBe(30000)
    expect(state.clips[0].approved).toBe(true)
  })

  it('starts export from review', () => {
    const state = wizardReducer(makeState({ screen: 'review' }), { type: 'START_EXPORT' })
    expect(state.screen).toBe('export')
    expect(state.exportStage).toBe('cutting')
  })

  it('completes export with outputDir', () => {
    const state = wizardReducer(makeState({ screen: 'export', exportStage: 'cutting' }), {
      type: 'EXPORT_DONE',
      outputDir: '/output/folder'
    })
    expect(state.exportStage).toBe('done')
    expect(state.outputDir).toBe('/output/folder')
  })

  it('handles export error', () => {
    const state = wizardReducer(makeState({ screen: 'export', exportStage: 'cutting' }), {
      type: 'EXPORT_ERROR',
      error: 'ffmpeg failed'
    })
    expect(state.exportStage).toBe('error')
    expect(state.exportError).toBe('ffmpeg failed')
  })

  it('returns home and keeps settings', () => {
    const state = wizardReducer(
      makeState({
        screen: 'export',
        provider: 'openai',
        model: 'gpt-5-mini-2025-08-07',
        apiKey: 'sk-test',
        videoPath: '/video.mp4',
        outputDir: '/output',
        settingsLoaded: true,
        segments: [{ startMs: 0, endMs: 1000, text: 'hi' }]
      }),
      { type: 'GO_HOME' }
    )
    expect(state.screen).toBe('home')
    expect(state.provider).toBe('openai')
    expect(state.apiKey).toBe('sk-test')
    expect(state.settingsLoaded).toBe(true)
    expect(state.videoPath).toBeNull()
    expect(state.outputDir).toBeNull()
    expect(state.segments).toEqual([])
  })

  it('walks transcribe, find, review, and export without a forced earlier step', () => {
    let state = wizardReducer(makeState(), {
      type: 'OPEN_TOOL',
      tool: 'transcribe',
      videoPath: '/video.mp4'
    })
    state = wizardReducer(state, { type: 'START_TRANSCRIBE' })
    const segments: TranscriptSegment[] = [{ startMs: 0, endMs: 5000, text: 'Hello world' }]
    state = wizardReducer(state, { type: 'TRANSCRIBE_DONE', segments })
    expect(state.screen).toBe('transcribe-done')

    state = wizardReducer(state, { type: 'OPEN_TOOL', tool: 'find', videoPath: '/video.mp4' })
    state = wizardReducer(state, {
      type: 'PROJECT_LOADED',
      segments,
      clips: [],
      rawResponse: '',
      framing: {},
      captions: null,
      durationMs: 5000
    })
    expect(state.screen).toBe('find')

    const clips: ClipSegmentWithStatus[] = [
      { id: '0', title: 'Clip', startMs: 0, endMs: 5000, approved: true }
    ]
    state = wizardReducer(state, { type: 'ANALYZE_DONE', clips, rawResponse: '[]' })
    expect(state.screen).toBe('review')
    state = wizardReducer(state, { type: 'START_EXPORT' })
    state = wizardReducer(state, { type: 'EXPORT_DONE', outputDir: '/out' })
    expect(state.exportStage).toBe('done')
    expect(state.outputDir).toBe('/out')
  })
})
