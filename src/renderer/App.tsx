import { useReducer, useEffect, useCallback } from 'react'
import TitleBar from './TitleBar'
import type { PipelineStage, LLMProvider, ProgressUpdate } from '../shared/types'

// --- Default models per provider ---

const DEFAULT_MODELS: Record<LLMProvider, string> = {
  claude: 'claude-haiku-4-5-20251001',
  openai: 'gpt-5-mini-2025-08-07'
}

// --- State & Reducer ---

interface AppState {
  stage: PipelineStage
  provider: LLMProvider
  model: string
  apiKey: string
  videoPath: string | null
  message: string
  percent: number
  outputFolder: string | null
  settingsLoaded: boolean
}

type AppAction =
  | { type: 'SET_PROVIDER'; provider: LLMProvider }
  | { type: 'SET_MODEL'; model: string }
  | { type: 'SET_API_KEY'; apiKey: string }
  | { type: 'SET_VIDEO'; videoPath: string }
  | { type: 'PROGRESS'; update: ProgressUpdate }
  | { type: 'LOAD_SETTINGS'; provider: LLMProvider; model: string; apiKey: string }
  | { type: 'RESET' }

const initialState: AppState = {
  stage: 'idle',
  provider: 'claude',
  model: DEFAULT_MODELS.claude,
  apiKey: '',
  videoPath: null,
  message: '',
  percent: 0,
  outputFolder: null,
  settingsLoaded: false
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'SET_PROVIDER':
      return {
        ...state,
        provider: action.provider,
        model: DEFAULT_MODELS[action.provider]
      }
    case 'SET_MODEL':
      return { ...state, model: action.model }
    case 'SET_API_KEY':
      return { ...state, apiKey: action.apiKey }
    case 'SET_VIDEO':
      return { ...state, videoPath: action.videoPath }
    case 'PROGRESS': {
      const { stage, message, percent } = action.update
      if (stage === 'done') {
        return { ...state, stage: 'done', message: '', percent: 100, outputFolder: message }
      }
      if (stage === 'error') {
        return { ...state, stage: 'error', message, percent: 0 }
      }
      return { ...state, stage, message, percent }
    }
    case 'LOAD_SETTINGS':
      return {
        ...state,
        provider: action.provider,
        model: action.model,
        apiKey: action.apiKey,
        settingsLoaded: true
      }
    case 'RESET':
      return {
        ...initialState,
        provider: state.provider,
        model: state.model,
        apiKey: state.apiKey,
        settingsLoaded: state.settingsLoaded
      }
    default:
      return state
  }
}

// --- Stage labels ---

const STAGE_LABELS: Partial<Record<PipelineStage, string>> = {
  extracting: 'Extracting Audio',
  downloading: 'Downloading Model',
  transcribing: 'Transcribing',
  analyzing: 'Analyzing',
  cutting: 'Cutting Clips'
}

// --- Component ---

export default function App(): React.JSX.Element {
  const [state, dispatch] = useReducer(appReducer, initialState)
  const { stage, provider, model, apiKey, videoPath, message, percent, outputFolder, settingsLoaded } = state

  const isRunning =
    stage === 'extracting' ||
    stage === 'downloading' ||
    stage === 'transcribing' ||
    stage === 'analyzing' ||
    stage === 'cutting'

  const canStart = videoPath && apiKey.length > 0 && !isRunning && stage !== 'done'

  // Load settings on mount
  useEffect(() => {
    window.api.loadSettings().then((s) => {
      dispatch({ type: 'LOAD_SETTINGS', provider: s.provider, model: s.model, apiKey: s.apiKey })
    })
  }, [])

  // Persist settings when they change (only after initial load)
  useEffect(() => {
    if (!settingsLoaded) return
    window.api.saveSettings({ provider, model, apiKey })
  }, [provider, model, apiKey, settingsLoaded])

  // Listen for pipeline progress
  useEffect(() => {
    const unsubscribe = window.api.onProgress((update: ProgressUpdate) => {
      dispatch({ type: 'PROGRESS', update })
    })
    return unsubscribe
  }, [])

  const handleSelectVideo = useCallback(async () => {
    const path = await window.api.selectVideo()
    if (path) dispatch({ type: 'SET_VIDEO', videoPath: path })
  }, [])

  const handleStart = useCallback(async () => {
    if (!videoPath) return
    await window.api.runPipeline(videoPath, { provider, model, apiKey })
  }, [videoPath, provider, model, apiKey])

  const handleCancel = useCallback(() => {
    window.api.cancelPipeline()
  }, [])

  const handleOpenFolder = useCallback(() => {
    if (outputFolder) window.api.openFolder(outputFolder)
  }, [outputFolder])

  const handleReset = useCallback(() => {
    dispatch({ type: 'RESET' })
  }, [])

  return (
    <div className="relative flex flex-col min-h-screen bg-bg-base text-neutral-200 font-sans overflow-hidden"
      style={{ background: 'radial-gradient(ellipse at top, #12121e 0%, #08080f 60%)' }}
    >
      {/* Decorative background elements */}
      <div className="pointer-events-none absolute inset-0 overflow-hidden">
        {/* Accent glow — top right */}
        <div
          className="absolute -top-24 -right-24 w-72 h-72 rounded-full opacity-[0.06] blur-3xl"
          style={{ background: 'rgb(240, 154, 62)' }}
        />
        {/* Secondary glow — bottom left */}
        <div
          className="absolute -bottom-32 -left-32 w-80 h-80 rounded-full opacity-[0.04] blur-3xl"
          style={{ background: 'rgb(240, 154, 62)' }}
        />
        {/* Grid pattern overlay */}
        <div
          className="absolute inset-0 opacity-[0.03]"
          style={{
            backgroundImage:
              'linear-gradient(rgba(255,255,255,0.06) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.06) 1px, transparent 1px)',
            backgroundSize: '48px 48px'
          }}
        />
        {/* Film-strip dashes — left edge */}
        <div className="absolute top-20 left-3 flex flex-col gap-3 opacity-[0.06]">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="w-1 h-4 rounded-full bg-white" />
          ))}
        </div>
        {/* Film-strip dashes — right edge */}
        <div className="absolute top-20 right-3 flex flex-col gap-3 opacity-[0.06]">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="w-1 h-4 rounded-full bg-white" />
          ))}
        </div>
      </div>

      <TitleBar />

      <div className="relative flex flex-col items-center px-6 pb-10 pt-4 gap-6 max-w-lg mx-auto w-full flex-1">
        {/* Header */}
        <div className="flex items-center gap-3">
          {/* Scissors / slice icon */}
          <svg
            viewBox="0 0 32 32"
            fill="none"
            className="w-9 h-9 shrink-0"
            aria-hidden="true"
          >
            <rect x="2" y="6" width="28" height="20" rx="4" stroke="rgb(240,154,62)" strokeWidth="1.5" opacity="0.5" />
            <line x1="12" y1="6" x2="12" y2="26" stroke="rgb(240,154,62)" strokeWidth="1.5" strokeDasharray="3 2" />
            <polygon points="10,14 14,16 10,18" fill="rgb(240,154,62)" opacity="0.8" />
            <rect x="4" y="10" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
            <rect x="4" y="15" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
            <rect x="4" y="20" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
            <rect x="16" y="12" width="11" height="2" rx="1" fill="white" opacity="0.15" />
            <rect x="16" y="17" width="8" height="2" rx="1" fill="white" opacity="0.10" />
          </svg>
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">
              A1 <span style={{ color: 'rgb(240, 154, 62)' }}>Slice</span>
            </h1>
            <p className="text-xs text-neutral-500">Cut long videos into short reels</p>
          </div>
        </div>

        {/* Settings Card */}
        <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 space-y-4 shadow-lg">
          <div className="flex gap-3">
            <label className="flex flex-col gap-1.5 text-xs text-neutral-400 w-40">
              Provider
              <select
                value={provider}
                onChange={(e) =>
                  dispatch({ type: 'SET_PROVIDER', provider: e.target.value as LLMProvider })
                }
                disabled={isRunning}
                className="appearance-none bg-bg-input border border-white/12 rounded-lg pl-3 pr-8 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors disabled:opacity-40 select-chevron"
              >
                <option value="claude">Claude</option>
                <option value="openai">OpenAI</option>
              </select>
            </label>
            <label className="flex flex-col gap-1.5 text-xs text-neutral-400 flex-1">
              Model
              <input
                type="text"
                value={model}
                onChange={(e) => dispatch({ type: 'SET_MODEL', model: e.target.value })}
                disabled={isRunning}
                className="bg-bg-input border border-white/12 rounded-lg px-3 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors disabled:opacity-40"
              />
            </label>
          </div>
          <label className="flex flex-col gap-1.5 text-xs text-neutral-400">
            API Key
            <input
              type="password"
              placeholder="sk-..."
              value={apiKey}
              onChange={(e) => dispatch({ type: 'SET_API_KEY', apiKey: e.target.value })}
              disabled={isRunning}
              className="bg-bg-input border border-white/12 rounded-lg px-3 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors disabled:opacity-40"
            />
          </label>
        </div>

        {/* Video Card */}
        <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 space-y-4 shadow-lg">
          <div className="flex items-center gap-3">
            <button
              onClick={handleSelectVideo}
              disabled={isRunning}
              className="bg-bg-input border border-white/12 rounded-lg px-4 py-2 text-sm text-neutral-200 hover:border-white/25 transition-colors disabled:opacity-40 disabled:cursor-not-allowed shrink-0"
            >
              Choose File
            </button>
            {videoPath && (
              <span className="text-sm text-neutral-400 truncate">
                {videoPath.split(/[\\/]/).pop()}
              </span>
            )}
          </div>

          {!isRunning ? (
            <button
              onClick={handleStart}
              disabled={!canStart}
              className="w-full bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg py-2.5 text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-accent"
            >
              Start Processing
            </button>
          ) : (
            <button
              onClick={handleCancel}
              className="w-full bg-red-800 hover:bg-red-700 text-white font-medium rounded-lg py-2.5 text-sm transition-colors"
            >
              Cancel
            </button>
          )}
        </div>

        {/* Progress Card */}
        {isRunning && (
          <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 space-y-3 shadow-lg">
            <div className="flex items-center justify-between text-sm">
              <span className="text-neutral-300 font-medium">
                {STAGE_LABELS[stage] ?? stage}
              </span>
              <span className="text-neutral-500 tabular-nums">{percent}%</span>
            </div>
            <div className="w-full h-2 bg-bg-input rounded-full overflow-hidden">
              <div
                className="h-full bg-accent rounded-full transition-all duration-500 ease-out"
                style={{ width: `${percent}%` }}
              />
            </div>
            <p className="text-xs text-neutral-500">{message}</p>
          </div>
        )}

        {/* Done Card */}
        {stage === 'done' && (
          <div className="w-full bg-bg-card border border-emerald-800 rounded-2xl p-5 space-y-4 shadow-lg">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-emerald-400" />
              <span className="text-sm text-emerald-300 font-medium">Clips are ready</span>
            </div>
            <div className="flex gap-3">
              <button
                onClick={handleOpenFolder}
                className="flex-1 bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg py-2 text-sm transition-colors"
              >
                Open Folder
              </button>
              <button
                onClick={handleReset}
                className="flex-1 bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2 text-sm transition-colors"
              >
                Process More
              </button>
            </div>
          </div>
        )}

        {/* Error Card */}
        {stage === 'error' && (
          <div className="w-full bg-bg-card border border-red-900 rounded-2xl p-5 space-y-4 shadow-lg">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-red-400" />
              <span className="text-sm text-red-300 font-medium">Error</span>
            </div>
            <p className="text-sm text-neutral-400">{message}</p>
            <button
              onClick={handleReset}
              className="w-full bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2 text-sm transition-colors"
            >
              Try Again
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
