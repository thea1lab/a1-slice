import { useEffect, useState } from 'react'
import ClipCard from './ClipCard'
import type { ClipCrop, ClipSegmentWithStatus } from '../../shared/types'

interface StepReviewSlicesProps {
  clips: ClipSegmentWithStatus[]
  videoPath: string
  rawResponse?: string
  videoDurationMs?: number
  onToggle: (id: string) => void
  onUpdateClipTimes?: (id: string, startMs: number, endMs: number) => void
  onUpdateClipCrop?: (id: string, crop: ClipCrop) => void
  onSlice: () => void
}

export default function StepReviewSlices({
  clips,
  videoPath,
  rawResponse,
  videoDurationMs,
  onToggle,
  onUpdateClipTimes,
  onUpdateClipCrop,
  onSlice
}: StepReviewSlicesProps): React.JSX.Element {
  const [index, setIndex] = useState(0)
  const approvedCount = clips.filter((c) => c.approved).length
  const safeIndex = clips.length === 0 ? 0 : Math.min(index, clips.length - 1)
  const clip = clips[safeIndex]

  useEffect(() => {
    if (index > clips.length - 1) setIndex(Math.max(0, clips.length - 1))
  }, [clips.length, index])

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return
      if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setIndex((i) => Math.max(0, i - 1))
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        setIndex((i) => Math.min(clips.length - 1, i + 1))
      } else if (e.key === 'd' || e.key === 'D') {
        if (!clip) return
        const discarding = clip.approved
        onToggle(clip.id)
        if (discarding && safeIndex < clips.length - 1) setIndex(safeIndex + 1)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [clip, clips.length, onToggle, safeIndex])

  if (!clip) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <p className="text-sm text-white/50">No clips to review.</p>
      </div>
    )
  }

  const discard = (): void => {
    const discarding = clip.approved
    onToggle(clip.id)
    if (discarding && safeIndex < clips.length - 1) setIndex(safeIndex + 1)
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 w-full bg-black">
      <div className="relative flex-1 min-h-0">
        <div className={`h-full min-h-0 flex flex-col ${clip.approved ? '' : 'opacity-50'}`}>
          <ClipCard
            clip={clip}
            clips={clips}
            index={safeIndex}
            videoPath={videoPath}
            videoDurationMs={videoDurationMs}
            onUpdateTimes={onUpdateClipTimes}
            onUpdateCrop={onUpdateClipCrop}
          />
        </div>

        <button
          type="button"
          aria-label="Previous clip"
          disabled={safeIndex === 0}
          onClick={() => setIndex(safeIndex - 1)}
          className="absolute left-2 top-[42%] z-20 w-9 h-9 text-white/40 hover:text-white disabled:opacity-0"
        >
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
            <path d="M12.5 4.5L7 10l5.5 5.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="Next clip"
          disabled={safeIndex >= clips.length - 1}
          onClick={() => setIndex(safeIndex + 1)}
          className="absolute right-2 top-[42%] z-20 w-9 h-9 text-white/40 hover:text-white disabled:opacity-0"
        >
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
            <path d="M7.5 4.5L13 10l-5.5 5.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>

      <div className="shrink-0 flex items-center gap-3 px-4 py-2.5 bg-black">
        <span className="text-xs tabular-nums text-white/45 min-w-[3.5rem]">
          {safeIndex + 1} / {clips.length}
        </span>
        <span className="text-xs text-white/35">
          {approvedCount} kept
        </span>
        <div className="flex-1" />
        <button
          type="button"
          onClick={discard}
          className="text-sm text-white/50 hover:text-white px-2 py-1"
        >
          {clip.approved ? 'Drop' : 'Keep'}
        </button>
        <button
          type="button"
          onClick={onSlice}
          disabled={approvedCount === 0}
          className="text-sm font-medium text-black bg-accent hover:bg-accent-hover rounded px-3.5 py-1.5 disabled:opacity-35 disabled:hover:bg-accent"
        >
          Export {approvedCount}
        </button>
      </div>

      {rawResponse && (
        <details className="px-4 pb-2 bg-black">
          <summary className="text-[11px] text-white/30 cursor-pointer hover:text-white/50">
            Model output
          </summary>
          <pre className="mt-2 p-2 text-[11px] text-white/45 overflow-x-auto max-h-32 overflow-y-auto whitespace-pre-wrap break-words">
            {rawResponse}
          </pre>
        </details>
      )}
    </div>
  )
}
