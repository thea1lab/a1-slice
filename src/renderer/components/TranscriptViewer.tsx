import type { TranscriptSegment } from '../../shared/types'

function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const m = Math.floor(totalSeconds / 60)
  const s = totalSeconds % 60
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

interface TranscriptViewerProps {
  segments: TranscriptSegment[]
}

export default function TranscriptViewer({
  segments
}: TranscriptViewerProps): React.JSX.Element {
  return (
    <div className="max-h-64 overflow-y-auto custom-scrollbar space-y-1 pr-2">
      {segments.map((seg, i) => (
        <div key={i} className="flex gap-3 text-sm py-1.5">
          <span className="text-neutral-500 tabular-nums shrink-0 text-xs pt-0.5">
            {formatTime(seg.startMs)}
          </span>
          <span className="text-neutral-300">{seg.text.trim()}</span>
        </div>
      ))}
    </div>
  )
}
