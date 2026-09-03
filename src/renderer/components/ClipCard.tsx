import React, { useEffect, useRef, useState } from 'react'
import VideoPreview from './VideoPreview'
import type { ClipCrop, ClipSegmentWithStatus, CropRatio } from '../../shared/types'
import { DEFAULT_CROP } from '../../shared/types'
import { clamp, cropFromPrevious } from '../../shared/crop'
import { PREVIEW_PAD_MS } from '../../shared/previewUrl'

const MIN_CLIP_MS = 500

const RATIO_CHIPS: { key: CropRatio; label: string; shape: string }[] = [
  { key: 'original', label: 'Original', shape: 'w-3.5 h-2' },
  { key: '4:3', label: '4:3', shape: 'w-3.5 h-2.5' },
  { key: '9:16', label: '9:16', shape: 'w-2 h-3.5' },
  { key: '1:1', label: '1:1', shape: 'w-2.5 h-2.5' }
]

function formatDuration(startMs: number, endMs: number): string {
  const totalSeconds = Math.round((endMs - startMs) / 1000)
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  if (h > 0) return `${h}h ${m}m ${s}s`
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

function msToHMMSSs(ms: number): string {
  const totalSeconds = ms / 1000
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${s.toFixed(1).padStart(4, '0')}`
  }
  return `${String(m).padStart(2, '0')}:${s.toFixed(1).padStart(4, '0')}`
}

function parseTime(value: string): number | null {
  const hMatch = value.trim().match(/^(\d+):(\d+):(\d+(?:\.\d+)?)$/)
  if (hMatch) {
    const h = parseInt(hMatch[1], 10)
    const m = parseInt(hMatch[2], 10)
    const s = parseFloat(hMatch[3])
    if (isNaN(h) || isNaN(m) || isNaN(s) || m >= 60 || s >= 60) return null
    const ms = (h * 3600 + m * 60 + s) * 1000
    return ms >= 0 ? Math.round(ms) : null
  }
  const mMatch = value.trim().match(/^(\d+):(\d+(?:\.\d+)?)$/)
  if (!mMatch) return null
  const m = parseInt(mMatch[1], 10)
  const s = parseFloat(mMatch[2])
  if (isNaN(m) || isNaN(s) || s >= 60) return null
  const ms = (m * 60 + s) * 1000
  return ms >= 0 ? Math.round(ms) : null
}

interface ClipCardProps {
  clip: ClipSegmentWithStatus
  clips: ClipSegmentWithStatus[]
  index: number
  videoPath: string
  videoDurationMs?: number
  onUpdateTimes?: (id: string, startMs: number, endMs: number) => void
  onUpdateCrop?: (id: string, crop: ClipCrop) => void
}

const ClipCard = React.memo(function ClipCard({
  clip,
  clips,
  index,
  videoPath,
  videoDurationMs,
  onUpdateTimes,
  onUpdateCrop
}: ClipCardProps): React.JSX.Element {
  const crop = clip.crop ?? DEFAULT_CROP
  const [startInput, setStartInput] = useState(msToHMMSSs(clip.startMs))
  const [endInput, setEndInput] = useState(msToHMMSSs(clip.endMs))
  const [playMs, setPlayMs] = useState(clip.startMs)
  const [seekToMs, setSeekToMs] = useState(clip.startMs)
  const [seekNonce, setSeekNonce] = useState(0)
  const [toast, setToast] = useState<string | null>(null)
  const trackRef = useRef<HTMLDivElement>(null)

  const maxMs =
    videoDurationMs && videoDurationMs > 0
      ? videoDurationMs
      : Math.max(clip.endMs + 120000, clip.startMs + 120000)

  const srcStart = Math.max(0, clip.startMs - PREVIEW_PAD_MS)
  const srcEnd = Math.min(maxMs, clip.endMs + PREVIEW_PAD_MS)
  const span = Math.max(1, srcEnd - srcStart)

  useEffect(() => {
    setStartInput(msToHMMSSs(clip.startMs))
  }, [clip.startMs])

  useEffect(() => {
    setEndInput(msToHMMSSs(clip.endMs))
  }, [clip.endMs])

  useEffect(() => {
    setPlayMs(clip.startMs)
    setSeekToMs(clip.startMs)
    setSeekNonce((n) => n + 1)
    setToast(null)
  }, [clip.id, clip.startMs])

  const seek = (ms: number): void => {
    const next = clamp(ms, clip.startMs, clip.endMs)
    setPlayMs(next)
    setSeekToMs(next)
    setSeekNonce((n) => n + 1)
  }

  const handleStartBlur = (): void => {
    const ms = parseTime(startInput)
    if (ms !== null && ms < clip.endMs && ms <= maxMs && onUpdateTimes) {
      onUpdateTimes(clip.id, ms, clip.endMs)
    } else {
      setStartInput(msToHMMSSs(clip.startMs))
    }
  }

  const handleEndBlur = (): void => {
    const ms = parseTime(endInput)
    if (ms !== null && ms > clip.startMs && ms <= maxMs && onUpdateTimes) {
      onUpdateTimes(clip.id, clip.startMs, ms)
    } else {
      setEndInput(msToHMMSSs(clip.endMs))
    }
  }

  const setRatio = (ratio: CropRatio): void => {
    if (!onUpdateCrop || crop.ratio === ratio) return
    const { crop: next, fromTitle } = cropFromPrevious(clips, index, ratio)
    onUpdateCrop(clip.id, next)
    if (fromTitle) {
      setToast(`Crop copied from “${fromTitle}”`)
      window.setTimeout(() => setToast(null), 1800)
    } else {
      setToast(null)
    }
  }

  const msToX = (ms: number, width: number): number => ((ms - srcStart) / span) * width
  const xToMs = (x: number, width: number): number => srcStart + (x / width) * span

  const pointerMs = (clientX: number): number => {
    const track = trackRef.current
    if (!track) return clip.startMs
    const rect = track.getBoundingClientRect()
    return xToMs(clamp(clientX - rect.left, 0, rect.width), rect.width)
  }

  const onTimelinePointer = (
    e: React.PointerEvent,
    mode: 'in' | 'out' | 'play' | 'auto'
  ): void => {
    if (!onUpdateTimes) return
    e.preventDefault()
    e.stopPropagation()
    let current: 'in' | 'out' | 'play'
    if (mode === 'auto') {
      const ms = pointerMs(e.clientX)
      if (Math.abs(ms - clip.startMs) < 400) current = 'in'
      else if (Math.abs(ms - clip.endMs) < 400) current = 'out'
      else current = 'play'
    } else {
      current = mode
    }

    const apply = (clientX: number): void => {
      const ms = pointerMs(clientX)
      if (current === 'in') {
        const startMs = clamp(ms, srcStart, clip.endMs - MIN_CLIP_MS)
        onUpdateTimes(clip.id, startMs, clip.endMs)
        seek(startMs)
      } else if (current === 'out') {
        const endMs = clamp(ms, clip.startMs + MIN_CLIP_MS, srcEnd)
        onUpdateTimes(clip.id, clip.startMs, endMs)
        seek(endMs)
      } else {
        seek(ms)
      }
    }

    apply(e.clientX)
    const move = (ev: PointerEvent): void => apply(ev.clientX)
    const up = (): void => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  const trackWidth = 100
  const x0 = clamp(msToX(clip.startMs, trackWidth), 0, trackWidth)
  const x1 = clamp(msToX(clip.endMs, trackWidth), 0, trackWidth)
  const xp = clamp(msToX(playMs, trackWidth), 0, trackWidth)

  return (
    <div className="flex flex-col">
      <div className="relative">
        <VideoPreview
          videoPath={videoPath}
          startMs={clip.startMs}
          endMs={clip.endMs}
          videoDurationMs={videoDurationMs}
          crop={crop}
          onCropChange={onUpdateCrop ? (next) => onUpdateCrop(clip.id, next) : undefined}
          onPlayheadMs={setPlayMs}
          seekToMs={seekToMs}
          seekNonce={seekNonce}
          editor
        />
        {toast && (
          <div className="absolute left-3 top-3 z-20 bg-bg-card/95 border border-accent/45 text-accent text-xs font-semibold px-2.5 py-1.5 rounded-lg">
            {toast}
          </div>
        )}
      </div>

      <div className="p-4 flex flex-col gap-3.5">
        <div className="flex items-baseline justify-between gap-3">
          <h3 className="text-lg font-semibold text-neutral-200 min-w-0 truncate">
            {clip.title}
            {clip.topic && (
              <span className="ml-2 text-sm font-normal text-neutral-500">{clip.topic}</span>
            )}
          </h3>
        </div>

        {onUpdateTimes && (
          <div className="flex flex-col gap-2">
            <div
              className="relative h-10 bg-bg-input border border-white/12 rounded-[10px] cursor-pointer select-none touch-none"
              onPointerDown={(e) => onTimelinePointer(e, 'auto')}
            >
              <div
                ref={trackRef}
                className="absolute left-[18px] right-[18px] top-1/2 h-1.5 -mt-[3px] bg-white/10 rounded-full"
              >
                <div
                  className="absolute top-0 h-full bg-accent rounded-full"
                  style={{ left: `${x0}%`, width: `${Math.max(1, x1 - x0)}%` }}
                />
                <button
                  type="button"
                  aria-label="Start"
                  className="absolute top-1/2 w-3.5 h-[26px] -ml-[7px] -mt-[13px] bg-accent border-2 border-bg-base rounded-[5px] cursor-ew-resize z-[2]"
                  style={{ left: `${x0}%` }}
                  onPointerDown={(e) => onTimelinePointer(e, 'in')}
                />
                <button
                  type="button"
                  aria-label="End"
                  className="absolute top-1/2 w-3.5 h-[26px] -ml-[7px] -mt-[13px] bg-accent border-2 border-bg-base rounded-[5px] cursor-ew-resize z-[2]"
                  style={{ left: `${x1}%` }}
                  onPointerDown={(e) => onTimelinePointer(e, 'out')}
                />
                <div
                  className="absolute top-[-10px] bottom-[-10px] w-0.5 -ml-px bg-white rounded-sm z-[3] pointer-events-none"
                  style={{ left: `${xp}%` }}
                />
              </div>
            </div>
            <div className="flex items-center justify-between gap-2">
              <input
                type="text"
                value={startInput}
                onChange={(e) => setStartInput(e.target.value)}
                onBlur={handleStartBlur}
                aria-label="Start time"
                className="w-[4.5rem] h-7 bg-bg-input border border-white/12 rounded-lg px-2 text-neutral-300 text-xs text-center font-mono"
              />
              <span className="text-xs font-semibold text-accent bg-accent/12 border border-accent/30 rounded-full px-2.5 py-1">
                {formatDuration(clip.startMs, clip.endMs)}
              </span>
              <input
                type="text"
                value={endInput}
                onChange={(e) => setEndInput(e.target.value)}
                onBlur={handleEndBlur}
                aria-label="End time"
                className="w-[4.5rem] h-7 bg-bg-input border border-white/12 rounded-lg px-2 text-neutral-300 text-xs text-center font-mono"
              />
            </div>
          </div>
        )}

        {onUpdateCrop && (
          <div className="flex items-center gap-2.5 flex-wrap">
            <span className="w-11 shrink-0 text-xs font-medium text-neutral-400">Crop</span>
            <div className="flex gap-2 flex-wrap">
              {RATIO_CHIPS.map((chip) => (
                <button
                  key={chip.key}
                  type="button"
                  onClick={() => setRatio(chip.key)}
                  className={`inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-semibold border transition-colors ${
                    crop.ratio === chip.key
                      ? 'bg-accent/15 border-accent text-accent'
                      : 'border-white/12 text-neutral-300 hover:border-accent/45'
                  }`}
                >
                  <span className={`${chip.shape} border-[1.5px] border-current rounded-[2px]`} />
                  {chip.label}
                </button>
              ))}
            </div>
          </div>
        )}

        <p className="text-[11px] text-neutral-500">
          {crop.ratio === 'original'
            ? 'Original keeps the full frame. Pick a ratio, then drag the window onto the subject.'
            : 'Drag the window to place it. Use + / −, a corner, or scroll to zoom.'}
        </p>
      </div>
    </div>
  )
})

export default ClipCard
