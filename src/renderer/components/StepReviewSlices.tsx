import ClipCard from './ClipCard'
import type { ClipSegmentWithStatus } from '../../shared/types'

interface StepReviewSlicesProps {
  clips: ClipSegmentWithStatus[]
  videoPath: string
  rawResponse?: string
  videoDurationMs?: number
  onToggle: (id: string) => void
  onUpdateClipTimes?: (id: string, startMs: number, endMs: number) => void
  onSlice: () => void
}

export default function StepReviewSlices({
  clips,
  videoPath,
  rawResponse,
  videoDurationMs,
  onToggle,
  onUpdateClipTimes,
  onSlice
}: StepReviewSlicesProps): React.JSX.Element {
  const approvedCount = clips.filter((c) => c.approved).length

  return (
    <div className="flex flex-col gap-6 w-full py-4 px-2">
      {/* Header */}
      <div className="flex items-center justify-between px-2">
        <h2 className="text-sm font-medium text-neutral-300">
          {approvedCount} of {clips.length} clips selected
        </h2>
        <button
          onClick={onSlice}
          disabled={approvedCount === 0}
          className="bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg px-6 py-2.5 text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-accent"
        >
          Slice {approvedCount} Clip{approvedCount !== 1 ? 's' : ''}
        </button>
      </div>

      {/* Clip list */}
      <div className="flex flex-col gap-3">
        {clips.map((clip) => (
          <ClipCard
            key={clip.id}
            clip={clip}
            videoPath={videoPath}
            videoDurationMs={videoDurationMs}
            onToggle={onToggle}
            onUpdateTimes={onUpdateClipTimes}
          />
        ))}
      </div>

      {/* Raw LLM response */}
      {rawResponse && (
        <details className="px-2">
          <summary className="text-xs text-neutral-500 cursor-pointer hover:text-neutral-400 transition-colors">
            View raw LLM response
          </summary>
          <pre className="mt-2 p-3 bg-bg-input border border-white/7 rounded-lg text-xs text-neutral-400 overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap break-words">
            {rawResponse}
          </pre>
        </details>
      )}
    </div>
  )
}
