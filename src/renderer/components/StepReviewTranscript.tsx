import TranscriptViewer from './TranscriptViewer'
import ProgressBar from './ProgressBar'
import type { TranscriptSegment, LLMProvider } from '../../shared/types'

const DEFAULT_MODELS: Record<LLMProvider, string> = {
  claude: 'claude-haiku-4-5-20251001',
  openai: 'gpt-5-mini-2025-08-07'
}

interface StepReviewTranscriptProps {
  segments: TranscriptSegment[]
  provider: LLMProvider
  model: string
  apiKey: string
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  error: string | null
  onProviderChange: (provider: LLMProvider) => void
  onModelChange: (model: string) => void
  onApiKeyChange: (apiKey: string) => void
  onAnalyze: () => void
  onCancel: () => void
}

export default function StepReviewTranscript({
  segments,
  provider,
  model,
  apiKey,
  analyzing,
  analyzePercent,
  analyzeMessage,
  error,
  onProviderChange,
  onModelChange,
  onApiKeyChange,
  onAnalyze,
  onCancel
}: StepReviewTranscriptProps): React.JSX.Element {
  const canAnalyze = apiKey.length > 0 && !analyzing

  return (
    <div className="flex flex-col gap-6 max-w-2xl mx-auto w-full py-4">
      {/* Transcript */}
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 shadow-lg space-y-3">
        <h2 className="text-sm font-medium text-neutral-300">
          Transcript ({segments.length} segments)
        </h2>
        <TranscriptViewer segments={segments} />
      </div>

      {/* LLM Settings */}
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-5 space-y-4 shadow-lg">
        <h2 className="text-sm font-medium text-neutral-300">LLM Settings</h2>
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
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-red-400" />
            <span className="text-sm text-red-300">{error}</span>
          </div>
        )}

        <div className="flex gap-3">
          {!analyzing ? (
            <button
              onClick={onAnalyze}
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
      </div>
    </div>
  )
}
