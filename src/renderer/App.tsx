import { useReducer, useEffect, useCallback } from 'react'
import TitleBar from './TitleBar'
import Stepper from './components/Stepper'
import StepSelectVideo from './components/StepSelectVideo'
import StepTranscribe from './components/StepTranscribe'
import StepReviewTranscript from './components/StepReviewTranscript'
import StepReviewSlices from './components/StepReviewSlices'
import StepExport from './components/StepExport'
import type {
  WizardStep,
  PipelineStage,
  LLMProvider,
  VideoLanguage,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegmentWithStatus,
  ClipCrop
} from '../shared/types'
import { DEFAULT_MODELS, DEFAULT_CROP } from '../shared/types'
import { refineClipBounds } from '../shared/clipBounds'

// --- State & Reducer ---

interface WizardState {
  currentStep: WizardStep
  completedSteps: WizardStep[]

  // Settings
  provider: LLMProvider
  model: string
  apiKey: string
  apiKeys: Record<string, string>
  userHint: string
  language: VideoLanguage
  entropyThold: number
  maxContext: number
  beamSize: number
  temperatureInc: number
  settingsLoaded: boolean

  // Step 1: Select video
  videoPath: string | null

  // Step 2: Transcribe
  transcribeStage: PipelineStage
  transcribeMessage: string
  transcribePercent: number
  transcribeError: string | null

  // Step 3: Review transcript
  segments: TranscriptSegment[]
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  analyzeError: string | null

  // Step 4: Review slices
  clips: ClipSegmentWithStatus[]
  rawResponse: string

  // Step 5: Export
  exportStage: PipelineStage
  exportMessage: string
  exportPercent: number
  exportError: string | null
  outputDir: string | null
}

type WizardAction =
  | { type: 'LOAD_SETTINGS'; provider: LLMProvider; model: string; apiKey: string; apiKeys?: Record<string, string>; userHint: string; language: VideoLanguage; entropyThold: number; maxContext: number; beamSize: number; temperatureInc: number }
  | { type: 'SET_PROVIDER'; provider: LLMProvider }
  | { type: 'SET_MODEL'; model: string }
  | { type: 'SET_API_KEY'; apiKey: string }
  | { type: 'SET_USER_HINT'; userHint: string }
  | { type: 'SET_LANGUAGE'; language: VideoLanguage }
  | { type: 'SET_ENTROPY_THOLD'; entropyThold: number }
  | { type: 'SET_MAX_CONTEXT'; maxContext: number }
  | { type: 'SET_BEAM_SIZE'; beamSize: number }
  | { type: 'SET_TEMPERATURE_INC'; temperatureInc: number }
  | { type: 'SET_VIDEO'; videoPath: string }
  | { type: 'GO_TO_STEP'; step: WizardStep }
  | { type: 'START_TRANSCRIBE' }
  | { type: 'TRANSCRIBE_PROGRESS'; update: ProgressUpdate }
  | { type: 'TRANSCRIBE_DONE'; segments: TranscriptSegment[] }
  | { type: 'TRANSCRIBE_ERROR'; error: string }
  | { type: 'START_ANALYZE' }
  | { type: 'ANALYZE_PROGRESS'; update: ProgressUpdate }
  | { type: 'ANALYZE_DONE'; clips: ClipSegmentWithStatus[]; rawResponse: string }
  | { type: 'ANALYZE_ERROR'; error: string }
  | { type: 'TOGGLE_CLIP'; id: string }
  | { type: 'UPDATE_CLIP_TIMES'; id: string; startMs: number; endMs: number }
  | { type: 'UPDATE_CLIP_CROP'; id: string; crop: ClipCrop }
  | { type: 'LOAD_CACHED_TRANSCRIPT'; segments: TranscriptSegment[] }
  | { type: 'START_EXPORT' }
  | { type: 'EXPORT_PROGRESS'; update: ProgressUpdate }
  | { type: 'EXPORT_DONE'; outputDir: string }
  | { type: 'EXPORT_ERROR'; error: string }
  | { type: 'RESET' }

const initialState: WizardState = {
  currentStep: 'select',
  completedSteps: [],

  provider: 'claude',
  model: DEFAULT_MODELS.claude,
  apiKey: '',
  apiKeys: {},
  userHint: '',
  language: 'auto',
  entropyThold: 2.8,
  maxContext: 64,
  beamSize: 5,
  temperatureInc: 0.1,
  settingsLoaded: false,

  videoPath: null,

  transcribeStage: 'idle',
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

  exportStage: 'idle',
  exportMessage: '',
  exportPercent: 0,
  exportError: null,
  outputDir: null
}

function addCompleted(steps: WizardStep[], step: WizardStep): WizardStep[] {
  return steps.includes(step) ? steps : [...steps, step]
}

function storedApiKey(
  apiKeys: Record<string, string>,
  provider: LLMProvider
): string {
  return apiKeys[provider] ?? ''
}

function withApiKey(
  apiKeys: Record<string, string>,
  provider: LLMProvider,
  apiKey: string
): Record<string, string> {
  if (!apiKey) {
    if (!(provider in apiKeys)) return apiKeys
    const next = { ...apiKeys }
    delete next[provider]
    return next
  }
  return { ...apiKeys, [provider]: apiKey }
}

export function wizardReducer(state: WizardState, action: WizardAction): WizardState {
  switch (action.type) {
    case 'LOAD_SETTINGS': {
      const apiKeys = { ...(action.apiKeys ?? {}) }
      if (action.apiKey && !apiKeys[action.provider]) {
        apiKeys[action.provider] = action.apiKey
      }
      return {
        ...state,
        provider: action.provider,
        model: action.model,
        apiKey: apiKeys[action.provider] ?? action.apiKey,
        apiKeys,
        userHint: action.userHint,
        language: action.language,
        entropyThold: action.entropyThold,
        maxContext: action.maxContext,
        beamSize: action.beamSize,
        temperatureInc: action.temperatureInc,
        settingsLoaded: true
      }
    }
    case 'SET_PROVIDER': {
      if (action.provider === state.provider) return state
      const model = DEFAULT_MODELS[action.provider]
      return {
        ...state,
        provider: action.provider,
        model,
        apiKey: storedApiKey(state.apiKeys, action.provider)
      }
    }
    case 'SET_MODEL':
      return { ...state, model: action.model }
    case 'SET_API_KEY':
      return {
        ...state,
        apiKey: action.apiKey,
        apiKeys: withApiKey(state.apiKeys, state.provider, action.apiKey)
      }
    case 'SET_USER_HINT':
      return { ...state, userHint: action.userHint }
    case 'SET_LANGUAGE':
      return { ...state, language: action.language }
    case 'SET_ENTROPY_THOLD':
      return { ...state, entropyThold: action.entropyThold }
    case 'SET_MAX_CONTEXT':
      return { ...state, maxContext: action.maxContext }
    case 'SET_BEAM_SIZE':
      return { ...state, beamSize: action.beamSize }
    case 'SET_TEMPERATURE_INC':
      return { ...state, temperatureInc: action.temperatureInc }
    case 'SET_VIDEO':
      return { ...state, videoPath: action.videoPath }
    case 'GO_TO_STEP':
      return { ...state, currentStep: action.step }

    // Transcription
    case 'START_TRANSCRIBE':
      return {
        ...state,
        currentStep: 'transcribe',
        completedSteps: addCompleted(state.completedSteps, 'select'),
        transcribeStage: 'extracting',
        transcribeMessage: '',
        transcribePercent: 0,
        transcribeError: null
      }
    case 'TRANSCRIBE_PROGRESS':
      return {
        ...state,
        transcribeStage: action.update.stage,
        transcribeMessage: action.update.message,
        transcribePercent: action.update.percent
      }
    case 'TRANSCRIBE_DONE':
      return {
        ...state,
        currentStep: 'review-transcript',
        completedSteps: addCompleted(state.completedSteps, 'transcribe'),
        transcribeStage: 'done',
        transcribePercent: 100,
        segments: action.segments
      }
    case 'TRANSCRIBE_ERROR':
      return {
        ...state,
        transcribeStage: 'error',
        transcribeError: action.error
      }

    // Analysis
    case 'START_ANALYZE':
      return {
        ...state,
        analyzing: true,
        analyzePercent: 0,
        analyzeMessage: 'AI is picking the best clips...',
        analyzeError: null
      }
    case 'ANALYZE_PROGRESS':
      return {
        ...state,
        analyzePercent: action.update.percent,
        analyzeMessage: action.update.message
      }
    case 'ANALYZE_DONE':
      return {
        ...state,
        currentStep: 'review-slices',
        completedSteps: addCompleted(state.completedSteps, 'review-transcript'),
        analyzing: false,
        analyzePercent: 100,
        clips: action.clips,
        rawResponse: action.rawResponse
      }
    case 'ANALYZE_ERROR':
      return {
        ...state,
        analyzing: false,
        analyzeError: action.error
      }

    // Clip toggle
    case 'TOGGLE_CLIP':
      return {
        ...state,
        clips: state.clips.map((c) =>
          c.id === action.id ? { ...c, approved: !c.approved } : c
        )
      }

    // Clip time editing
    case 'UPDATE_CLIP_TIMES':
      return {
        ...state,
        clips: state.clips.map((c) =>
          c.id === action.id
            ? { ...c, startMs: action.startMs, endMs: action.endMs }
            : c
        )
      }

    case 'UPDATE_CLIP_CROP':
      return {
        ...state,
        clips: state.clips.map((c) =>
          c.id === action.id ? { ...c, crop: action.crop } : c
        )
      }

    // Load cached transcript (skip transcription)
    case 'LOAD_CACHED_TRANSCRIPT':
      return {
        ...state,
        currentStep: 'review-transcript',
        completedSteps: addCompleted(
          addCompleted(state.completedSteps, 'select'),
          'transcribe'
        ),
        transcribeStage: 'done',
        transcribePercent: 100,
        segments: action.segments
      }

    // Export
    case 'START_EXPORT':
      return {
        ...state,
        currentStep: 'export',
        completedSteps: addCompleted(state.completedSteps, 'review-slices'),
        exportStage: 'cutting',
        exportMessage: '',
        exportPercent: 0,
        exportError: null,
        outputDir: null
      }
    case 'EXPORT_PROGRESS':
      return {
        ...state,
        exportStage: action.update.stage,
        exportMessage: action.update.message,
        exportPercent: action.update.percent
      }
    case 'EXPORT_DONE':
      return {
        ...state,
        exportStage: 'done',
        exportPercent: 100,
        outputDir: action.outputDir,
        completedSteps: addCompleted(state.completedSteps, 'export')
      }
    case 'EXPORT_ERROR':
      return {
        ...state,
        exportStage: 'error',
        exportError: action.error
      }

    case 'RESET':
      return {
        ...initialState,
        provider: state.provider,
        model: state.model,
        apiKey: state.apiKey,
        apiKeys: state.apiKeys,
        userHint: state.userHint,
        language: state.language,
        entropyThold: state.entropyThold,
        maxContext: state.maxContext,
        beamSize: state.beamSize,
        temperatureInc: state.temperatureInc,
        settingsLoaded: state.settingsLoaded
      }

    default:
      return state
  }
}

// --- Component ---

export default function App(): React.JSX.Element {
  const [state, dispatch] = useReducer(wizardReducer, initialState)

  useEffect(() => {
    if (state.videoPath) {
      window.api.allowVideoPath(state.videoPath)
    }
  }, [state.videoPath])

  // Load settings on mount
  useEffect(() => {
    window.api.loadSettings().then((s) => {
      dispatch({
        type: 'LOAD_SETTINGS',
        provider: s.provider,
        model: s.model,
        apiKey: s.apiKey,
        apiKeys: s.apiKeys,
        userHint: s.userHint,
        language: s.language,
        entropyThold: s.entropyThold ?? 2.8,
        maxContext: s.maxContext ?? 64,
        beamSize: s.beamSize ?? 5,
        temperatureInc: s.temperatureInc ?? 0.1
      })
    })
  }, [])

  // Persist settings when they change
  useEffect(() => {
    if (!state.settingsLoaded) return
    window.api.saveSettings({
      provider: state.provider,
      model: state.model,
      apiKey: state.apiKey,
      apiKeys: state.apiKeys,
      userHint: state.userHint,
      language: state.language,
      entropyThold: state.entropyThold,
      maxContext: state.maxContext,
      beamSize: state.beamSize,
      temperatureInc: state.temperatureInc
    })
  }, [state.provider, state.model, state.apiKey, state.apiKeys, state.userHint, state.language, state.entropyThold, state.maxContext, state.beamSize, state.temperatureInc, state.settingsLoaded])

  // Listen for pipeline progress — route to the correct step
  useEffect(() => {
    const unsubscribe = window.api.onProgress((update: ProgressUpdate) => {
      if (state.currentStep === 'transcribe') {
        dispatch({ type: 'TRANSCRIBE_PROGRESS', update })
      } else if (
        state.currentStep === 'review-transcript' &&
        state.analyzing
      ) {
        dispatch({ type: 'ANALYZE_PROGRESS', update })
      } else if (state.currentStep === 'export') {
        dispatch({ type: 'EXPORT_PROGRESS', update })
      }
    })
    return unsubscribe
  }, [state.currentStep, state.analyzing])

  const completedSet = new Set(state.completedSteps)

  // --- Handlers ---

  const handleSelectVideo = useCallback(async () => {
    const path = await window.api.selectVideo()
    if (path) dispatch({ type: 'SET_VIDEO', videoPath: path })
  }, [])

  const handleStartTranscribe = useCallback(async () => {
    if (!state.videoPath) return
    dispatch({ type: 'START_TRANSCRIBE' })
    const result = await window.api.transcribeVideo(state.videoPath, state.language, state.entropyThold, state.maxContext, state.beamSize, state.temperatureInc)
    if (result.success && result.segments) {
      dispatch({ type: 'TRANSCRIBE_DONE', segments: result.segments })
    } else {
      dispatch({
        type: 'TRANSCRIBE_ERROR',
        error: result.error || 'Transcription failed'
      })
    }
  }, [state.videoPath, state.language, state.entropyThold, state.maxContext, state.beamSize, state.temperatureInc])

  const clampClips = useCallback(
    (clips: { title: string; startMs: number; endMs: number }[], rawResponse: string) => {
      const maxMs = state.segments.length > 0
        ? state.segments[state.segments.length - 1].endMs
        : Infinity
      const clipsWithStatus = clips
        .map((clip, i) => {
          const refined = refineClipBounds(
            {
              ...clip,
              startMs: clip.startMs,
              endMs: clip.endMs
            },
            state.segments
          )
          return {
            ...refined,
            startMs: Math.max(0, Math.min(refined.startMs, maxMs)),
            endMs: Math.max(0, Math.min(refined.endMs, maxMs)),
            id: String(i),
            approved: true,
            crop: { ...DEFAULT_CROP }
          }
        })
        .filter((clip) => clip.endMs > clip.startMs)
      if (clipsWithStatus.length === 0) {
        dispatch({
          type: 'ANALYZE_ERROR',
          error: 'No clips found. Try different suggestions or a different video.'
        })
        return
      }
      dispatch({ type: 'ANALYZE_DONE', clips: clipsWithStatus, rawResponse })
    },
    [state.segments]
  )

  const handleAnalyze = useCallback(async (userHint?: string) => {
    if (!state.videoPath) return
    dispatch({ type: 'START_ANALYZE' })
    const result = await window.api.analyzeTranscript(state.videoPath, state.segments, {
      provider: state.provider,
      model: state.model,
      apiKey: state.apiKey,
      apiKeys: state.apiKeys,
      userHint: state.userHint,
      language: state.language,
      entropyThold: state.entropyThold,
      maxContext: state.maxContext,
      beamSize: state.beamSize,
      temperatureInc: state.temperatureInc
    }, userHint)
    if (result.success && result.clips) {
      clampClips(result.clips, result.rawResponse || '')
    } else {
      dispatch({
        type: 'ANALYZE_ERROR',
        error: result.error || 'Analysis failed'
      })
    }
  }, [state.videoPath, state.segments, state.provider, state.model, state.apiKey, clampClips])

  const handleLoadCachedAnalysis = useCallback(
    (clips: { title: string; startMs: number; endMs: number }[], rawResponse: string) => {
      clampClips(clips, rawResponse)
    },
    [clampClips]
  )

  const handleSlice = useCallback(async () => {
    if (!state.videoPath) return
    const approved = state.clips.filter((c) => c.approved)
    dispatch({ type: 'START_EXPORT' })
    const result = await window.api.cutClips(
      state.videoPath,
      approved.map(({ title, startMs, endMs, crop }) => ({ title, startMs, endMs, crop })),
      state.segments
    )
    if (result.success && result.outputDir) {
      dispatch({ type: 'EXPORT_DONE', outputDir: result.outputDir })
    } else {
      dispatch({
        type: 'EXPORT_ERROR',
        error: result.error || 'Export failed'
      })
    }
  }, [state.videoPath, state.clips, state.segments])

  const handleCancel = useCallback(() => {
    window.api.cancelPipeline()
  }, [])

  const handleOpenFolder = useCallback(() => {
    if (state.outputDir) window.api.openFolder(state.outputDir)
  }, [state.outputDir])

  const handleReset = useCallback(() => {
    dispatch({ type: 'RESET' })
  }, [])

  const handleStepClick = useCallback((step: WizardStep) => {
    dispatch({ type: 'GO_TO_STEP', step })
  }, [])

  const handleLoadCachedTranscript = useCallback(
    (segments: TranscriptSegment[]) => {
      dispatch({ type: 'LOAD_CACHED_TRANSCRIPT', segments })
    },
    []
  )

  const handleUpdateClipTimes = useCallback(
    (id: string, startMs: number, endMs: number) => {
      dispatch({ type: 'UPDATE_CLIP_TIMES', id, startMs, endMs })
    },
    []
  )

  const handleUpdateClipCrop = useCallback(
    (id: string, crop: ClipCrop) => {
      dispatch({ type: 'UPDATE_CLIP_CROP', id, crop })
    },
    []
  )

  // --- Render current step ---

  function renderStep(): React.JSX.Element {
    switch (state.currentStep) {
      case 'select':
        return (
          <StepSelectVideo
            videoPath={state.videoPath}
            language={state.language}
            entropyThold={state.entropyThold}
            maxContext={state.maxContext}
            beamSize={state.beamSize}
            temperatureInc={state.temperatureInc}
            onLanguageChange={(l) => dispatch({ type: 'SET_LANGUAGE', language: l })}
            onEntropyTholdChange={(v) => dispatch({ type: 'SET_ENTROPY_THOLD', entropyThold: v })}
            onMaxContextChange={(v) => dispatch({ type: 'SET_MAX_CONTEXT', maxContext: v })}
            onBeamSizeChange={(v) => dispatch({ type: 'SET_BEAM_SIZE', beamSize: v })}
            onTemperatureIncChange={(v) => dispatch({ type: 'SET_TEMPERATURE_INC', temperatureInc: v })}
            onSelectVideo={handleSelectVideo}
            onNext={handleStartTranscribe}
            onLoadCachedTranscript={handleLoadCachedTranscript}
          />
        )
      case 'transcribe':
        return (
          <StepTranscribe
            message={state.transcribeMessage}
            percent={state.transcribePercent}
            error={state.transcribeError}
            onCancel={handleCancel}
            onRetry={handleStartTranscribe}
          />
        )
      case 'review-transcript':
        return (
          <StepReviewTranscript
            segments={state.segments}
            videoPath={state.videoPath}
            provider={state.provider}
            model={state.model}
            apiKey={state.apiKey}
            userHint={state.userHint}
            analyzing={state.analyzing}
            analyzePercent={state.analyzePercent}
            analyzeMessage={state.analyzeMessage}
            error={state.analyzeError}
            onProviderChange={(p) =>
              dispatch({ type: 'SET_PROVIDER', provider: p })
            }
            onModelChange={(m) => dispatch({ type: 'SET_MODEL', model: m })}
            onApiKeyChange={(k) =>
              dispatch({ type: 'SET_API_KEY', apiKey: k })
            }
            onUserHintChange={(h) =>
              dispatch({ type: 'SET_USER_HINT', userHint: h })
            }
            onAnalyze={handleAnalyze}
            onLoadCachedAnalysis={handleLoadCachedAnalysis}
            onCancel={handleCancel}
          />
        )
      case 'review-slices':
        return (
          <StepReviewSlices
            clips={state.clips}
            videoPath={state.videoPath!}
            rawResponse={state.rawResponse}
            videoDurationMs={state.segments.length > 0 ? state.segments[state.segments.length - 1].endMs : 0}
            onToggle={(id) => dispatch({ type: 'TOGGLE_CLIP', id })}
            onUpdateClipTimes={handleUpdateClipTimes}
            onUpdateClipCrop={handleUpdateClipCrop}
            onSlice={handleSlice}
          />
        )
      case 'export':
        return (
          <StepExport
            stage={state.exportStage}
            message={state.exportMessage}
            percent={state.exportPercent}
            outputDir={state.outputDir}
            error={state.exportError}
            onOpenFolder={handleOpenFolder}
            onStartOver={handleReset}
            onCancel={handleCancel}
          />
        )
    }
  }

  return (
    <div
      className="relative flex flex-col h-screen bg-bg-base text-neutral-200 font-sans overflow-hidden"
      style={{
        background: 'radial-gradient(ellipse at top, #12121e 0%, #08080f 60%)'
      }}
    >
      {/* Decorative background elements */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        <div
          className="absolute -top-24 -right-24 w-72 h-72 rounded-full opacity-[0.06] blur-3xl"
          style={{ background: 'rgb(240, 154, 62)' }}
        />
        <div
          className="absolute -bottom-32 -left-32 w-80 h-80 rounded-full opacity-[0.04] blur-3xl"
          style={{ background: 'rgb(240, 154, 62)' }}
        />
        <div
          className="absolute inset-0 opacity-[0.03]"
          style={{
            backgroundImage:
              'linear-gradient(rgba(255,255,255,0.06) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.06) 1px, transparent 1px)',
            backgroundSize: '48px 48px'
          }}
        />
        <div className="absolute top-20 left-3 flex flex-col gap-3 opacity-[0.06]">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="w-1 h-4 rounded-full bg-white" />
          ))}
        </div>
        <div className="absolute top-20 right-3 flex flex-col gap-3 opacity-[0.06]">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="w-1 h-4 rounded-full bg-white" />
          ))}
        </div>
      </div>

      <TitleBar />

      <Stepper
        currentStep={state.currentStep}
        completedSteps={completedSet}
        onStepClick={handleStepClick}
      />

      <div className="relative flex flex-col flex-1 px-6 pb-10 overflow-y-auto custom-scrollbar">
        {renderStep()}
      </div>
    </div>
  )
}
