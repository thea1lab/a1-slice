import { useState, useEffect, useCallback } from 'react'
import TranscriptViewer from './TranscriptViewer'
import ProgressBar from './ProgressBar'
import type { TranscriptSegment, ClipSegment, LLMProvider } from '../../shared/types'

const DEFAULT_MODELS: Record<LLMProvider, string> = {
  claude: 'claude-haiku-4-5',
  openai: 'gpt-5-mini-2025-08-07'
}

interface StepReviewTranscriptProps {
  segments: TranscriptSegment[]
  videoPath: string | null
  provider: LLMProvider
  model: string
  apiKey: string
  userHint: string
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  error: string | null
  onProviderChange: (provider: LLMProvider) => void
  onModelChange: (model: string) => void
  onApiKeyChange: (apiKey: string) => void
  onUserHintChange: (userHint: string) => void
  onAnalyze: (userHint?: string) => void
  onLoadCachedAnalysis?: (clips: ClipSegment[], rawResponse: string) => void
  onCancel: () => void
}

export default function StepReviewTranscript({
  segments,
  videoPath,
  provider,
  model,
  apiKey,
  userHint,
  analyzing,
  analyzePercent,
  analyzeMessage,
  error,
  onProviderChange,
  onModelChange,
  onApiKeyChange,
  onUserHintChange,
  onAnalyze,
  onLoadCachedAnalysis,
  onCancel
}: StepReviewTranscriptProps): React.JSX.Element {
  const canAnalyze = apiKey.length > 0 && !analyzing

  const handleAnalyze = useCallback(() => {
    onAnalyze(userHint.trim() || undefined)
  }, [onAnalyze, userHint]) as React.MouseEventHandler<HTMLButtonElement>

  const [cachedAnalysis, setCachedAnalysis] = useState<{
    clips: ClipSegment[]
    rawResponse: string
  } | null>(null)

  useEffect(() => {
    if (!videoPath) {
      setCachedAnalysis(null)
      return
    }
    window.api
      .checkAnalysis(videoPath)
      .then((result) => {
        setCachedAnalysis(
          result.found && result.clips
            ? { clips: result.clips, rawResponse: result.rawResponse || '' }
            : null
        )
      })
      .catch(() => setCachedAnalysis(null))
  }, [videoPath])

  return (
    <div className="flex flex-col gap-6 max-w-2xl mx-auto w-full py-4">
      {/* Transcript */}
      <h2 className="text-base font-semibold text-neutral-200 -mb-3">
        Transcript ({segments.length} segments)
      </h2>
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 shadow-lg space-y-3">
        <TranscriptViewer segments={segments} />
      </div>

      {/* LLM Settings */}
      <h2 className="text-base font-semibold text-neutral-200 -mb-3">LLM Settings</h2>
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 space-y-4 shadow-lg">
        <div className="flex gap-3">
          <label className="flex flex-col gap-1.5 text-xs text-neutral-400 w-40">
            Provider
            <select
              value={provider}
              onChange={(e) => {
                const p = e.target.value as LLMProvider
                onProviderChange(p)
                onModelChange(DEFAULT_MODELS[p])
              }}
              disabled={analyzing}
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
              onChange={(e) => onModelChange(e.target.value)}
              disabled={analyzing}
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
            onChange={(e) => onApiKeyChange(e.target.value)}
            disabled={analyzing}
            className="bg-bg-input border border-white/12 rounded-lg px-3 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors disabled:opacity-40"
          />
        </label>

        {/* User suggestions */}
        <label className="flex flex-col gap-1.5 text-xs text-neutral-400">
          Suggestions for the AI (optional)
          <textarea
            value={userHint}
            onChange={(e) => onUserHintChange(e.target.value)}
            disabled={analyzing}
            placeholder="e.g. &quot;Pick 2 clips focused on the demo moments&quot; or &quot;I want clips about the bug detection feature&quot;"
            rows={2}
            className="bg-bg-input border border-white/12 rounded-lg px-3 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors disabled:opacity-40 resize-none"
          />
          <span className="text-[11px] text-neutral-500">
            Suggest themes, number of clips, or what parts to focus on.
          </span>
        </label>

        {/* Analysis progress or error */}
        {analyzing && (
          <div className="space-y-2">
            <ProgressBar
              percent={analyzePercent}
              label="Analyzing"
              sublabel={analyzeMessage}
            />
          </div>
        )}

        {error && (
          <div className="bg-red-950/40 border border-red-500/20 rounded-lg p-3 space-y-1.5">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-red-400 shrink-0" />
              <span className="text-sm font-medium text-red-300">Analysis failed</span>
            </div>
            <pre className="text-xs text-red-300/70 whitespace-pre-wrap break-all overflow-x-auto max-h-32 overflow-y-auto pl-4">{error}</pre>
          </div>
        )}

        {/* Cached analysis prompt */}
        {!analyzing && cachedAnalysis && onLoadCachedAnalysis && (
          <div className="bg-bg-input border border-accent/20 rounded-lg p-4 space-y-3">
            <p className="text-sm text-neutral-300">
              Found a previous analysis for this video ({cachedAnalysis.clips.length} clips).
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => onLoadCachedAnalysis(cachedAnalysis.clips, cachedAnalysis.rawResponse)}
                className="bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg px-4 py-2 text-sm transition-colors"
              >
                Use Previous Analysis
              </button>
              <button
                onClick={handleAnalyze}
                disabled={!canAnalyze}
                className="bg-bg-input border border-white/12 rounded-lg px-4 py-2 text-sm text-neutral-200 hover:border-white/25 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              >
                Re-analyze
              </button>
            </div>
          </div>
        )}

        {/* Normal analyze button (hidden when cached analysis is available) */}
        {(!cachedAnalysis || !onLoadCachedAnalysis) && (
          <div className="flex gap-3">
            {!analyzing ? (
              <button
                onClick={handleAnalyze}
                disabled={!canAnalyze}
                className="flex-1 bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg py-2.5 text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-accent"
              >
                Analyze Transcript
              </button>
            ) : (
              <button
                onClick={onCancel}
                className="flex-1 bg-red-800 hover:bg-red-700 text-white font-medium rounded-lg py-2.5 text-sm transition-colors"
              >
                Cancel
              </button>
            )}
          </div>
        )}

        {/* Cancel button while analyzing (always visible) */}
        {analyzing && cachedAnalysis && onLoadCachedAnalysis && (
          <div className="flex gap-3">
            <button
              onClick={onCancel}
              className="flex-1 bg-red-800 hover:bg-red-700 text-white font-medium rounded-lg py-2.5 text-sm transition-colors"
            >
              Cancel
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
