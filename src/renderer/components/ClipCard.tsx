import VideoPreview from './VideoPreview'
import type { ClipSegmentWithStatus } from '../../shared/types'

function formatDuration(startMs: number, endMs: number): string {
  const totalSeconds = Math.round((endMs - startMs) / 1000)
  const m = Math.floor(totalSeconds / 60)
  const s = totalSeconds % 60
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

interface ClipCardProps {
  clip: ClipSegmentWithStatus
  videoPath: string
  onToggle: (id: string) => void
}

export default function ClipCard({
  clip,
  videoPath,
  onToggle
}: ClipCardProps): React.JSX.Element {
  return (
    <div
      className={`bg-bg-card border rounded-2xl overflow-hidden transition-colors ${
        clip.approved ? 'border-accent/30' : 'border-white/7 opacity-60'
      }`}
    >
      <VideoPreview
        videoPath={videoPath}
        startMs={clip.startMs}
        endMs={clip.endMs}
      />
      <div className="p-4 space-y-3">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <h3 className="text-sm font-medium text-neutral-200 truncate">
              {clip.title}
            </h3>
            <p className="text-xs text-neutral-500 mt-0.5">
              {formatDuration(clip.startMs, clip.endMs)}
            </p>
          </div>
          <button
            onClick={() => onToggle(clip.id)}
            className={`shrink-0 w-10 h-6 rounded-full transition-colors relative ${
              clip.approved ? 'bg-accent' : 'bg-white/10'
            }`}
          >
            <div
              className={`absolute top-1 w-4 h-4 rounded-full bg-white transition-all ${
                clip.approved ? 'left-5' : 'left-1'
              }`}
            />
          </button>
        </div>
      </div>
    </div>
  )
}
