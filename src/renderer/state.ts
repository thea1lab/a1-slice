import type {
  CaptionProject,
  ClipCrop,
  ClipSegmentWithStatus,
  CropRatio,
  LLMProvider,
  PipelineStage,
  ProgressUpdate,
  Screen,
  SubtitleExport,
  ToolId,
  TranscriptSegment,
  VideoLanguage
} from '../shared/types'
import { DEFAULT_CROP, DEFAULT_MODELS } from '../shared/types'

export interface WizardState {
  screen: Screen
  returnTo: 'find' | 'captions' | null
  projectReady: boolean

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

  videoPath: string | null
  videoDurationMs: number

  transcribeStage: PipelineStage
  transcribeMessage: string
  transcribePercent: number
  transcribeError: string | null

  segments: TranscriptSegment[]
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  analyzeError: string | null

  clips: ClipSegmentWithStatus[]
  rawResponse: string
  framing: Partial<Record<CropRatio, ClipCrop>>
  captions: CaptionProject | null
  exportSubtitles: SubtitleExport

  exportStage: PipelineStage
  exportMessage: string
  exportPercent: number
  exportError: string | null
  outputDir: string | null
}

export type WizardAction =
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
  | { type: 'OPEN_TOOL'; tool: ToolId; videoPath: string }
  | { type: 'ENTER_TOOL'; tool: ToolId }
  | { type: 'PROJECT_LOADED'; segments: TranscriptSegment[]; clips: ClipSegmentWithStatus[]; rawResponse: string; framing: Partial<Record<CropRatio, ClipCrop>>; captions: CaptionProject | null; durationMs: number }
  | { type: 'SHOW_SCREEN'; screen: Screen }
  | { type: 'PREPARE_TRANSCRIBE'; returnTo: 'find' | 'captions' }
  | { type: 'SET_RETURN_TO'; returnTo: 'find' | 'captions' | null }
  | { type: 'GO_HOME' }
  | { type: 'START_TRANSCRIBE' }
  | { type: 'TRANSCRIBE_PROGRESS'; update: ProgressUpdate }
  | { type: 'TRANSCRIBE_DONE'; segments: TranscriptSegment[] }
  | { type: 'TRANSCRIBE_ERROR'; error: string }
  | { type: 'USE_SAVED_TRANSCRIPT' }
  | { type: 'START_ANALYZE' }
  | { type: 'ANALYZE_PROGRESS'; update: ProgressUpdate }
  | { type: 'ANALYZE_DONE'; clips: ClipSegmentWithStatus[]; rawResponse: string }
  | { type: 'ANALYZE_ERROR'; error: string }
  | { type: 'TOGGLE_CLIP'; id: string }
  | { type: 'UPDATE_CLIP_TIMES'; id: string; startMs: number; endMs: number }
  | { type: 'UPDATE_CLIP_CROP'; id: string; crop: ClipCrop }
  | { type: 'ADD_CLIP' }
  | { type: 'SET_FRAMING'; framing: Partial<Record<CropRatio, ClipCrop>> }
  | { type: 'SET_CAPTIONS'; captions: CaptionProject }
  | { type: 'SET_EXPORT_SUBTITLES'; exportSubtitles: SubtitleExport }
  | { type: 'START_EXPORT' }
  | { type: 'START_RENDER' }
  | { type: 'EXPORT_PROGRESS'; update: ProgressUpdate }
  | { type: 'EXPORT_DONE'; outputDir: string }
  | { type: 'EXPORT_ERROR'; error: string }

export const initialWizardState: WizardState = {
  screen: 'home',
  returnTo: null,
  projectReady: false,

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
  videoDurationMs: 0,

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
  framing: {},
  captions: null,
  exportSubtitles: 'srt',

  exportStage: 'idle',
  exportMessage: '',
  exportPercent: 0,
  exportError: null,
  outputDir: null
}

function storedApiKey(apiKeys: Record<string, string>, provider: LLMProvider): string {
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

function screenForTool(tool: ToolId): Screen {
  if (tool === 'transcribe') return 'transcribe'
  if (tool === 'find') return 'find'
  if (tool === 'reframe') return 'reframe'
  return 'captions'
}

function beginTool(state: WizardState, screen: Screen, videoPath: string | null): WizardState {
  return {
    ...state,
    screen,
    videoPath,
    returnTo: null,
    projectReady: false,
    segments: [],
    clips: [],
    rawResponse: '',
    framing: {},
    captions: null,
    videoDurationMs: 0,
    transcribeStage: 'idle',
    transcribeMessage: '',
    transcribePercent: 0,
    transcribeError: null,
    analyzing: false,
    analyzeError: null,
    exportStage: 'idle',
    exportMessage: '',
    exportPercent: 0,
    exportError: null,
    outputDir: null
  }
}

function screenAfterTranscript(state: WizardState): Screen {
  if (state.returnTo === 'find') return 'find'
  if (state.returnTo === 'captions') return 'captions'
  return 'transcribe-done'
}

function keepSettings(state: WizardState): WizardState {
  return {
    ...initialWizardState,
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
      return {
        ...state,
        provider: action.provider,
        model: DEFAULT_MODELS[action.provider],
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
    case 'ENTER_TOOL':
      return beginTool(state, screenForTool(action.tool), null)
    case 'OPEN_TOOL':
      return beginTool(state, screenForTool(action.tool), action.videoPath)
    case 'PROJECT_LOADED':
      return {
        ...state,
        projectReady: true,
        segments: action.segments,
        clips: action.clips,
        rawResponse: action.rawResponse,
        framing: action.framing,
        captions: action.captions,
        videoDurationMs: action.durationMs,
        screen:
          state.screen === 'find' && action.clips.length > 0 ? 'review' : state.screen
      }
    case 'SHOW_SCREEN':
      return { ...state, screen: action.screen }
    case 'PREPARE_TRANSCRIBE':
      return {
        ...state,
        screen: 'transcribe',
        returnTo: action.returnTo,
        transcribeStage: 'idle',
        transcribeError: null,
        transcribeMessage: '',
        transcribePercent: 0
      }
    case 'SET_RETURN_TO':
      return { ...state, returnTo: action.returnTo }
    case 'GO_HOME':
      return keepSettings(state)

    case 'START_TRANSCRIBE':
      return {
        ...state,
        screen: 'transcribe',
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
        screen: screenAfterTranscript(state),
        returnTo: null,
        transcribeStage: 'done',
        transcribePercent: 100,
        segments: action.segments,
        projectReady: true
      }
    case 'TRANSCRIBE_ERROR':
      return {
        ...state,
        transcribeStage: 'error',
        transcribeError: action.error
      }
    case 'USE_SAVED_TRANSCRIPT':
      return {
        ...state,
        screen: screenAfterTranscript(state),
        returnTo: null,
        transcribeStage: 'done',
        transcribePercent: 100
      }

    case 'START_ANALYZE':
      return {
        ...state,
        analyzing: true,
        analyzePercent: 0,
        analyzeMessage: 'Reading the transcript and marking clips…',
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
        screen: 'review',
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

    case 'TOGGLE_CLIP':
      return {
        ...state,
        clips: state.clips.map((clip) =>
          clip.id === action.id ? { ...clip, approved: !clip.approved } : clip
        )
      }
    case 'UPDATE_CLIP_TIMES':
      return {
        ...state,
        clips: state.clips.map((clip) =>
          clip.id === action.id
            ? { ...clip, startMs: action.startMs, endMs: action.endMs }
            : clip
        )
      }
    case 'UPDATE_CLIP_CROP':
      return {
        ...state,
        clips: state.clips.map((clip) =>
          clip.id === action.id ? { ...clip, crop: action.crop } : clip
        )
      }
    case 'ADD_CLIP': {
      const duration = state.videoDurationMs > 0 ? state.videoDurationMs : 60_000
      const endMs = Math.max(1000, Math.min(duration, 30_000))
      const clip: ClipSegmentWithStatus = {
        id: `manual-${state.clips.length}`,
        title: `Clip ${state.clips.length + 1}`,
        startMs: 0,
        endMs,
        approved: true,
        crop: { ...DEFAULT_CROP }
      }
      return { ...state, screen: 'review', clips: [...state.clips, clip] }
    }
    case 'SET_FRAMING':
      return { ...state, framing: action.framing }
    case 'SET_CAPTIONS':
      return { ...state, captions: action.captions }
    case 'SET_EXPORT_SUBTITLES':
      return { ...state, exportSubtitles: action.exportSubtitles }

    case 'START_EXPORT':
      return {
        ...state,
        screen: 'export',
        exportStage: 'cutting',
        exportMessage: '',
        exportPercent: 0,
        exportError: null,
        outputDir: null
      }
    case 'START_RENDER':
      return {
        ...state,
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
        outputDir: action.outputDir
      }
    case 'EXPORT_ERROR':
      return {
        ...state,
        exportStage: 'error',
        exportError: action.error
      }

    default:
      return state
  }
}
