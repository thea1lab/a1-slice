import { useEffect, useState } from 'react'
import ClipCard from '../components/ClipCard'
import SubtitleModePicker from '../components/SubtitleModePicker'
import type { ClipCrop, ClipSegmentWithStatus, CropRatio, SubtitleExport } from '../../shared/types'

interface ReviewScreenProps {
  clips: ClipSegmentWithStatus[]
  videoPath: string
  rawResponse?: string
  videoDurationMs?: number
  framing?: Partial<Record<CropRatio, ClipCrop>>
  exportSubtitles: SubtitleExport
  onToggle: (id: string) => void
  onUpdateClipTimes?: (id: string, startMs: number, endMs: number) => void
  onUpdateClipCrop?: (id: string, crop: ClipCrop) => void
  onExportSubtitles: (mode: SubtitleExport) => void
  onSlice: () => void
  onFindAgain: () => void
  onAddRange: () => void
}

export default function ReviewScreen({
  clips,
  videoPath,
  rawResponse,
  videoDurationMs,
  framing,
  exportSubtitles,
  onToggle,
  onUpdateClipTimes,
  onUpdateClipCrop,
  onExportSubtitles,
  onSlice,
  onFindAgain,
  onAddRange
}: ReviewScreenProps): React.JSX.Element {
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
        <p className="text-base text-white/60">No clips to review yet.</p>
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
            framing={framing}
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

      <div className="dock">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="dock-meta">
            <span>
              Clip {safeIndex + 1} of {clips.length}
            </span>
            <span>{approvedCount} kept</span>
          </p>
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-[15px] text-[#a8a8a8]">Subtitles on export</span>
            <SubtitleModePicker value={exportSubtitles} onChange={onExportSubtitles} />
          </div>
        </div>
        <p className="help">Keep the parts you want. Drop the rest, then export those clips.</p>
        <div className="actions">
          <button type="button" onClick={onFindAgain} className="btn btn-secondary">
            Find again
          </button>
          <button type="button" onClick={onAddRange} className="btn btn-secondary">
            Add a part
          </button>
          <button type="button" onClick={discard} className="btn btn-secondary">
            {clip.approved ? 'Drop this one' : 'Keep this one'}
          </button>
          <button
            type="button"
            onClick={onSlice}
            disabled={approvedCount === 0}
            className="btn btn-primary"
          >
            Export {approvedCount} {approvedCount === 1 ? 'clip' : 'clips'}
          </button>
        </div>
      </div>

      {rawResponse && (
        <details className="px-5 pb-3 bg-[#161616]">
          <summary className="text-sm text-white/40 cursor-pointer hover:text-white/70">
            Model notes
          </summary>
          <pre className="mt-2 p-2 text-sm text-white/55 overflow-x-auto max-h-32 overflow-y-auto whitespace-pre-wrap break-words">
            {rawResponse}
          </pre>
        </details>
      )}
    </div>
  )
}
