import { useState, useEffect, useCallback } from 'react'
import TranscriptViewer from './TranscriptViewer'
import ProgressBar from './ProgressBar'
import SegmentedChoice from './SegmentedChoice'
import type { TranscriptSegment, ClipSegment } from '../../shared/types'

interface ClipAgent {
  id: string
  label: string
}

interface FindClipsFormProps {
  segments: TranscriptSegment[]
  videoPath: string | null
  agents: ClipAgent[] | null
  agentId: string | null
  userHint: string
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  error: string | null
  onAgentChange: (agentId: string) => void
  onUserHintChange: (userHint: string) => void
  onAnalyze: (agentId: string, userHint?: string) => void
  onLoadCachedAnalysis?: (clips: ClipSegment[], rawResponse: string) => void
  onCancel: () => void
  onAddRange?: () => void
  onBackToClips?: () => void
}

export default function FindClipsForm({
  segments,
  videoPath,
  agents,
  agentId,
  userHint,
  analyzing,
  analyzePercent,
  analyzeMessage,
  error,
  onAgentChange,
  onUserHintChange,
  onAnalyze,
  onLoadCachedAnalysis,
  onCancel,
  onAddRange,
  onBackToClips
}: FindClipsFormProps): React.JSX.Element {
  const canAnalyze = Boolean(agentId) && !analyzing

  const handleAnalyze = useCallback(() => {
    if (!agentId) return
    onAnalyze(agentId, userHint.trim() || undefined)
  }, [agentId, onAnalyze, userHint]) as React.MouseEventHandler<HTMLButtonElement>

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

  const lines = segments.length

  return (
    <div className="sheet">
      <div>
        <h1 className="display">Find best parts</h1>
        <p className="lead mt-3">
          An agent reads the transcript and suggests clips, with a start and an end for each one.
          You can also mark a part yourself.
        </p>
      </div>
      {!analyzing && cachedAnalysis && onLoadCachedAnalysis && (
        <section className="notice">
          <p>
            A search is already saved for this video ({cachedAnalysis.clips.length}{' '}
            {cachedAnalysis.clips.length === 1 ? 'clip' : 'clips'}). Open it, or set up a new
            search below.
          </p>
          <div className="actions">
            <button
              type="button"
              onClick={() => onLoadCachedAnalysis(cachedAnalysis.clips, cachedAnalysis.rawResponse)}
              className="btn btn-primary btn-lg"
            >
              Open the saved search
            </button>
          </div>
        </section>
      )}

      <details className="w-full">
        <summary className="section-label cursor-pointer">
          Transcript · {lines} {lines === 1 ? 'line' : 'lines'}
        </summary>
        <div className="mt-3">
          <TranscriptViewer segments={segments} />
        </div>
      </details>

      {onAddRange && !analyzing && (
        <button type="button" onClick={onAddRange} className="btn btn-secondary">
          Mark a part yourself
        </button>
      )}

      <div className="block">
        <h2 className="section-label">Which agent</h2>
        <p className="help">
          The words are sent to this agent so it can suggest clips. The video stays on this
          computer.
        </p>
        {agents === null && <p className="help">Looking for agents on this computer…</p>}
        {agents !== null && agents.length === 0 && (
          <p className="help">This computer has no agent to run. You can still mark a part yourself.</p>
        )}
        {agents !== null && agents.length > 0 && agentId && (
          <SegmentedChoice
            label="Which agent"
            value={agentId}
            options={agents}
            disabled={analyzing}
            onChange={onAgentChange}
          />
        )}

        <label className="flex flex-col gap-2 w-full text-base text-[#a8a8a8]">
          What should it look for?
          <textarea
            value={userHint}
            onChange={(e) => onUserHintChange(e.target.value)}
            disabled={analyzing}
            placeholder="Two clips about the demo, or the part where the bug shows up."
            rows={3}
            className="field disabled:opacity-40 resize-none"
          />
          <span className="help">Optional. Name a theme, a number of clips, or a moment to keep.</span>
        </label>

        {analyzing && (
          <ProgressBar percent={analyzePercent} label="Reading the transcript" sublabel={analyzeMessage} />
        )}

        {error && (
          <div className="w-full bg-red-950/40 border border-red-500/20 rounded-lg p-4 space-y-2">
            <p className="text-base font-medium text-red-300">Could not find clips</p>
            <pre className="text-sm text-red-300/80 whitespace-pre-wrap break-words overflow-x-auto max-h-32 overflow-y-auto">
              {error}
            </pre>
          </div>
        )}

        {onBackToClips && !analyzing && (
          <button type="button" onClick={onBackToClips} className="btn btn-secondary">
            Back to the clips
          </button>
        )}

        {!analyzing ? (
          <button
            type="button"
            onClick={handleAnalyze}
            disabled={!canAnalyze}
            className="btn btn-primary btn-lg"
          >
            {cachedAnalysis ? 'Search again' : 'Find clips'}
          </button>
        ) : (
          <button type="button" onClick={onCancel} className="btn btn-danger">
            Cancel
          </button>
        )}
      </div>
    </div>
  )
}
