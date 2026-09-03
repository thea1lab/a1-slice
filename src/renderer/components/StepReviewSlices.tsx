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
      <div className="flex flex-col gap-4 w-full py-4 px-2">
        <p className="text-sm text-neutral-400">No clips to review.</p>
      </div>
    )
  }

  const discard = (): void => {
    const discarding = clip.approved
    onToggle(clip.id)
    if (discarding && safeIndex < clips.length - 1) setIndex(safeIndex + 1)
  }

  return (
    <div className="flex flex-col gap-4 w-full max-w-3xl mx-auto py-4 px-2">
      <div className="flex items-center justify-between px-1">
        <h2 className="text-base font-semibold text-neutral-200">
          {approvedCount} of {clips.length} clips kept
        </h2>
        <span className="text-sm text-neutral-500">
          Clip {safeIndex + 1} of {clips.length}
        </span>
      </div>

      <div className="grid grid-cols-[48px_minmax(0,1fr)_48px] items-center gap-2">
        <button
          type="button"
          aria-label="Previous clip"
          disabled={safeIndex === 0}
          onClick={() => setIndex(safeIndex - 1)}
          className="w-12 h-12 rounded-full border border-white/12 bg-bg-card text-neutral-200 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-25 disabled:hover:border-white/12 disabled:hover:text-neutral-200"
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M12.5 4.5L7 10l5.5 5.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>

        <article
          className={`bg-bg-card border rounded-2xl overflow-hidden ${
            clip.approved ? 'border-accent/30' : 'border-white/7 opacity-70'
          }`}
        >
          <ClipCard
            clip={clip}
            clips={clips}
            index={safeIndex}
            videoPath={videoPath}
            videoDurationMs={videoDurationMs}
            onUpdateTimes={onUpdateClipTimes}
            onUpdateCrop={onUpdateClipCrop}
          />
          <div className="flex items-stretch gap-2.5 px-4 pb-4">
            <button
              type="button"
              onClick={discard}
              className="shrink-0 rounded-xl px-4 py-3 text-sm font-semibold border border-white/14 text-neutral-300 hover:border-red-400/70 hover:text-red-400"
            >
              {clip.approved ? 'Discard clip' : 'Restore clip'}
            </button>
            <button
              type="button"
              onClick={onSlice}
              disabled={approvedCount === 0}
              className="flex-1 bg-accent hover:bg-accent-hover text-black font-bold rounded-xl px-5 py-3.5 text-base transition-colors disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-accent"
            >
              Save {approvedCount} clip{approvedCount !== 1 ? 's' : ''}
            </button>
          </div>
        </article>

        <button
          type="button"
          aria-label="Next clip"
          disabled={safeIndex >= clips.length - 1}
          onClick={() => setIndex(safeIndex + 1)}
          className="w-12 h-12 rounded-full border border-white/12 bg-bg-card text-neutral-200 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-25 disabled:hover:border-white/12 disabled:hover:text-neutral-200"
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M7.5 4.5L13 10l-5.5 5.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </div>

      <div className="flex justify-center gap-2">
        {clips.map((c, i) => (
          <button
            key={c.id}
            type="button"
            title={c.title}
            onClick={() => setIndex(i)}
            className={`w-2.5 h-2.5 rounded-full border-0 ${
              i === safeIndex
                ? 'bg-accent scale-125'
                : c.approved
                  ? 'bg-white/20'
                  : 'bg-transparent shadow-[inset_0_0_0_1.5px_rgba(240,113,120,0.8)]'
            }`}
          />
        ))}
      </div>

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
