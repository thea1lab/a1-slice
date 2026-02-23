import ClipCard from './ClipCard'
import type { ClipSegmentWithStatus } from '../../shared/types'

interface StepReviewSlicesProps {
  clips: ClipSegmentWithStatus[]
  videoPath: string
  onToggle: (id: string) => void
  onSlice: () => void
}

export default function StepReviewSlices({
  clips,
  videoPath,
  onToggle,
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

      {/* Clip grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {clips.map((clip) => (
          <ClipCard
            key={clip.id}
            clip={clip}
            videoPath={videoPath}
            onToggle={onToggle}
          />
        ))}
      </div>
    </div>
  )
}
