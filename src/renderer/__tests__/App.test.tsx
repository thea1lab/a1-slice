import { describe, it, expect } from 'vitest'
import { appReducer } from '../App'
import type { PipelineStage, LLMProvider } from '../../shared/types'

function makeState(overrides: Partial<{
  stage: PipelineStage
  provider: LLMProvider
  model: string
  apiKey: string
  videoPath: string | null
  message: string
  percent: number
  outputFolder: string | null
  settingsLoaded: boolean
}> = {}) {
  return {
    stage: 'idle' as PipelineStage,
    provider: 'claude' as LLMProvider,
    model: 'claude-sonnet-4-20250514',
    apiKey: '',
    videoPath: null,
    message: '',
    percent: 0,
    outputFolder: null,
    settingsLoaded: false,
    ...overrides
  }
}

describe('appReducer', () => {
  it('sets provider and resets model to provider default', () => {
    const state = appReducer(makeState(), {
      type: 'SET_PROVIDER',
      provider: 'gpt4o'
    })
    expect(state.provider).toBe('gpt4o')
    expect(state.model).toBe('gpt-4o')
  })

  it('sets provider back to claude with correct default model', () => {
    const state = appReducer(makeState({ provider: 'gpt4o', model: 'gpt-4o' }), {
      type: 'SET_PROVIDER',
      provider: 'claude'
    })
    expect(state.provider).toBe('claude')
    expect(state.model).toBe('claude-sonnet-4-20250514')
  })

  it('sets model', () => {
    const state = appReducer(makeState(), {
      type: 'SET_MODEL',
      model: 'claude-opus-4-20250514'
    })
    expect(state.model).toBe('claude-opus-4-20250514')
  })

  it('sets api key', () => {
    const state = appReducer(makeState(), {
      type: 'SET_API_KEY',
      apiKey: 'sk-test'
    })
    expect(state.apiKey).toBe('sk-test')
  })

  it('sets video path', () => {
    const state = appReducer(makeState(), {
      type: 'SET_VIDEO',
      videoPath: '/path/to/video.mp4'
    })
    expect(state.videoPath).toBe('/path/to/video.mp4')
  })

  it('handles progress update for extracting', () => {
    const state = appReducer(makeState(), {
      type: 'PROGRESS',
      update: { stage: 'extracting', message: 'Extracting...', percent: 42 }
    })
    expect(state.stage).toBe('extracting')
    expect(state.message).toBe('Extracting...')
    expect(state.percent).toBe(42)
  })

  it('handles progress update for done', () => {
    const state = appReducer(makeState({ stage: 'cutting' }), {
      type: 'PROGRESS',
      update: { stage: 'done', message: '/output/folder', percent: 100 }
    })
    expect(state.stage).toBe('done')
    expect(state.outputFolder).toBe('/output/folder')
    expect(state.percent).toBe(100)
  })

  it('handles progress update for error', () => {
    const state = appReducer(makeState({ stage: 'transcribing' }), {
      type: 'PROGRESS',
      update: { stage: 'error', message: 'Something broke', percent: 0 }
    })
    expect(state.stage).toBe('error')
    expect(state.message).toBe('Something broke')
  })

  it('loads settings from persistence', () => {
    const state = appReducer(makeState(), {
      type: 'LOAD_SETTINGS',
      provider: 'gpt4o',
      model: 'gpt-4o-mini',
      apiKey: 'sk-persisted'
    })
    expect(state.provider).toBe('gpt4o')
    expect(state.model).toBe('gpt-4o-mini')
    expect(state.apiKey).toBe('sk-persisted')
    expect(state.settingsLoaded).toBe(true)
  })

  it('resets state but preserves provider, model, apiKey, and settingsLoaded', () => {
    const state = appReducer(
      makeState({
        stage: 'done',
        provider: 'gpt4o',
        model: 'gpt-4o',
        apiKey: 'sk-test',
        videoPath: '/video.mp4',
        outputFolder: '/output',
        settingsLoaded: true
      }),
      { type: 'RESET' }
    )
    expect(state.stage).toBe('idle')
    expect(state.provider).toBe('gpt4o')
    expect(state.model).toBe('gpt-4o')
    expect(state.apiKey).toBe('sk-test')
    expect(state.settingsLoaded).toBe(true)
    expect(state.videoPath).toBeNull()
    expect(state.outputFolder).toBeNull()
  })

  it('transitions through full pipeline stages', () => {
    let state = makeState()

    state = appReducer(state, {
      type: 'SET_VIDEO',
      videoPath: '/video.mp4'
    })
    state = appReducer(state, {
      type: 'SET_API_KEY',
      apiKey: 'sk-key'
    })

    const stages: PipelineStage[] = [
      'extracting',
      'downloading',
      'transcribing',
      'analyzing',
      'cutting',
      'done'
    ]

    for (const stage of stages) {
      state = appReducer(state, {
        type: 'PROGRESS',
        update: { stage, message: stage === 'done' ? '/out' : `Stage: ${stage}`, percent: 50 }
      })
      expect(state.stage).toBe(stage)
    }

    expect(state.outputFolder).toBe('/out')
  })
})
