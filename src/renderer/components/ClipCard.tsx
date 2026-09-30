import React, { useEffect, useRef, useState } from 'react'
import VideoPreview, { type VideoPreviewHandle } from './VideoPreview'
import type { ClipCrop, ClipSegmentWithStatus, CropRatio } from '../../shared/types'
import { DEFAULT_CROP } from '../../shared/types'
import { clamp, CROP_PRESETS, cropFromPrevious } from '../../shared/crop'
import { nudgeEdge } from '../../shared/clipBounds'
import { PREVIEW_PAD_MS, pointerToSourceMs } from '../../shared/previewUrl'

const MIN_CLIP_MS = 500

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
  framing?: Partial<Record<CropRatio, ClipCrop>>
  onUpdateTimes?: (id: string, startMs: number, endMs: number) => void
  onUpdateCrop?: (id: string, crop: ClipCrop) => void
}

const ClipCard = React.memo(function ClipCard({
  clip,
  clips,
  index,
  videoPath,
  videoDurationMs,
  framing,
  onUpdateTimes,
  onUpdateCrop
}: ClipCardProps): React.JSX.Element {
  const crop = clip.crop ?? DEFAULT_CROP
  const playerRef = useRef<VideoPreviewHandle>(null)
  const rootRef = useRef<HTMLDivElement>(null)
  const [startInput, setStartInput] = useState(msToHMMSSs(clip.startMs))
  const [endInput, setEndInput] = useState(msToHMMSSs(clip.endMs))
  const [playMs, setPlayMs] = useState(clip.startMs)
  const [seekToMs, setSeekToMs] = useState(clip.startMs)
  const [seekNonce, setSeekNonce] = useState(0)
  const [toast, setToast] = useState<string | null>(null)
  const [playing, setPlaying] = useState(false)
  const [volume, setVolume] = useState(1)
  const [muted, setMuted] = useState(false)
  const [mediaSize, setMediaSize] = useState<{ width: number; height: number } | null>(null)
  const [fullscreen, setFullscreen] = useState(false)
  const trackRef = useRef<HTMLDivElement>(null)

  const maxMs =
    videoDurationMs && videoDurationMs > 0
      ? videoDurationMs
      : Math.max(clip.endMs + 120000, clip.startMs + 120000)

  const srcStart = Math.max(0, clip.startMs - PREVIEW_PAD_MS)
  const srcEnd = Math.min(maxMs, clip.endMs + PREVIEW_PAD_MS)
  const span = Math.max(1, srcEnd - srcStart)
  const effectiveVolume = muted ? 0 : volume
  const inTail = playMs < clip.startMs - 40 || playMs > clip.endMs + 40

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

  useEffect(() => {
    const onFs = (): void => {
      setFullscreen(document.fullscreenElement === rootRef.current)
    }
    document.addEventListener('fullscreenchange', onFs)
    return () => document.removeEventListener('fullscreenchange', onFs)
  }, [])

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return
      if (e.key === ' ' || e.code === 'Space') {
        e.preventDefault()
        playerRef.current?.togglePlay()
      } else if (e.key === 'f' || e.key === 'F') {
        e.preventDefault()
        toggleFullscreen()
      } else if (e.key === 'm' || e.key === 'M') {
        e.preventDefault()
        setMuted((on) => !on)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const seek = (ms: number): void => {
    const next = clamp(ms, srcStart, srcEnd)
    setPlayMs(next)
    setSeekToMs(next)
    setSeekNonce((n) => n + 1)
    playerRef.current?.seek(next)
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
    if (ratio !== 'original') {
      const saved = framing?.[ratio]
      if (saved) {
        onUpdateCrop(clip.id, { ...saved })
        setToast('Using the saved frame')
        window.setTimeout(() => setToast(null), 1800)
        return
      }
    }
    const { crop: next, fromTitle } = cropFromPrevious(clips, index, ratio)
    onUpdateCrop(clip.id, next)
    if (fromTitle) {
      setToast(`Frame copied from “${fromTitle}”`)
      window.setTimeout(() => setToast(null), 1800)
    } else {
      setToast(null)
    }
  }

  const nudge = (edge: 'start' | 'end', deltaMs: number): void => {
    if (!onUpdateTimes) return
    const next = nudgeEdge(clip.startMs, clip.endMs, edge, deltaMs, maxMs)
    onUpdateTimes(clip.id, next.startMs, next.endMs)
    seek(edge === 'start' ? next.startMs : next.endMs)
  }

  const msToX = (ms: number, width: number): number => ((ms - srcStart) / span) * width

  const pointerMs = (clientX: number): number => {
    const track = trackRef.current
    if (!track) return clip.startMs
    const rect = track.getBoundingClientRect()
    return pointerToSourceMs(clientX, rect.left, rect.width, srcStart, srcEnd)
  }

  const onTimelinePointer = (e: React.PointerEvent, mode: 'in' | 'out' | 'play'): void => {
    if ((mode === 'in' || mode === 'out') && !onUpdateTimes) return
    e.preventDefault()
    e.stopPropagation()

    const apply = (clientX: number): void => {
      const ms = pointerMs(clientX)
      if (mode === 'in' && onUpdateTimes) {
        const startMs = clamp(ms, srcStart, clip.endMs - MIN_CLIP_MS)
        onUpdateTimes(clip.id, startMs, clip.endMs)
        seek(startMs)
      } else if (mode === 'out' && onUpdateTimes) {
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

  const toggleFullscreen = (): void => {
    const root = rootRef.current
    if (!root) return
    if (document.fullscreenElement === root) void document.exitFullscreen()
    else void root.requestFullscreen()
  }

  const trackWidth = 100
  const x0 = clamp(msToX(clip.startMs, trackWidth), 0, trackWidth)
  const x1 = clamp(msToX(clip.endMs, trackWidth), 0, trackWidth)
  const xp = clamp(msToX(playMs, trackWidth), 0, trackWidth)

  return (
    <div ref={rootRef} data-player-root className="flex flex-col flex-1 min-h-0 bg-black">
      <div className="relative flex-1 min-h-0">
        <VideoPreview
          ref={playerRef}
          videoPath={videoPath}
          startMs={clip.startMs}
          endMs={clip.endMs}
          playStartMs={srcStart}
          playEndMs={srcEnd}
          videoDurationMs={videoDurationMs}
          crop={crop}
          volume={effectiveVolume}
          onCropChange={onUpdateCrop ? (next) => onUpdateCrop(clip.id, next) : undefined}
          onPlayheadMs={setPlayMs}
          onPlayingChange={setPlaying}
          onMediaInfo={setMediaSize}
          seekToMs={seekToMs}
          seekNonce={seekNonce}
          editor
          transport={false}
        />
        <div className="pointer-events-none absolute inset-x-0 top-0 z-10 bg-gradient-to-b from-black/55 to-transparent px-5 pt-4 pb-10">
          <h3 className="text-lg font-medium text-white truncate">{clip.title}</h3>
          <p className="text-sm text-white/70 truncate">
            {formatDuration(clip.startMs, clip.endMs)}
            {clip.topic ? ` · ${clip.topic}` : ''}
            {mediaSize ? ` · ${mediaSize.width}×${mediaSize.height}` : ''}
          </p>
        </div>
        {toast && (
          <div className="absolute left-4 top-14 z-20 bg-black/80 text-white text-xs px-2.5 py-1.5 rounded">
            {toast}
          </div>
        )}
        {inTail && (
          <div className="absolute right-4 top-3 z-20 text-sm text-white/80">
            Outside the clip
          </div>
        )}
      </div>

      <div className="shrink-0 bg-bg-base border-t border-white/8 px-4 pt-2 pb-3 flex flex-col gap-2">
        <div
          className="relative h-7 w-full cursor-pointer select-none touch-none"
          onPointerDown={(e) => onTimelinePointer(e, 'play')}
        >
          <div
            ref={trackRef}
            className="absolute left-0 right-0 w-full top-1/2 h-1 -mt-0.5 bg-white/15 rounded-full"
          >
              <div
                className="absolute top-0 h-full bg-accent rounded-full"
                style={{ left: `${x0}%`, width: `${Math.max(1, x1 - x0)}%` }}
              />
              {onUpdateTimes && (
                <>
                  <button
                    type="button"
                    aria-label="Start"
                    className="absolute top-1/2 w-2 h-5 -ml-1 -mt-2.5 bg-accent rounded-sm cursor-ew-resize z-[2]"
                    style={{ left: `${x0}%` }}
                    onPointerDown={(e) => onTimelinePointer(e, 'in')}
                  />
                  <button
                    type="button"
                    aria-label="End"
                    className="absolute top-1/2 w-2 h-5 -ml-1 -mt-2.5 bg-accent rounded-sm cursor-ew-resize z-[2]"
                    style={{ left: `${x1}%` }}
                    onPointerDown={(e) => onTimelinePointer(e, 'out')}
                  />
                </>
              )}
              <div
                className="absolute top-[-8px] bottom-[-8px] w-px bg-white z-[3] pointer-events-none"
                style={{ left: `${xp}%` }}
              />
            </div>
          </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => playerRef.current?.togglePlay()}
            className="w-8 h-8 shrink-0 text-white grid place-items-center hover:text-accent"
            aria-label={playing ? 'Pause' : 'Play'}
          >
            {playing ? (
              <svg width="14" height="14" viewBox="0 0 12 12" fill="currentColor">
                <rect x="2" y="1.5" width="2.5" height="9" rx="0.5" />
                <rect x="7.5" y="1.5" width="2.5" height="9" rx="0.5" />
              </svg>
            ) : (
              <svg width="14" height="14" viewBox="0 0 12 12" fill="currentColor">
                <path d="M3 1.5v9l7.5-4.5L3 1.5z" />
              </svg>
            )}
          </button>

          <span className="text-sm font-mono tabular-nums text-neutral-200 min-w-[4.6rem]">
            {msToHMMSSs(playMs)}
          </span>
          <span className="text-sm text-neutral-600">/</span>
          <span className="text-sm font-mono tabular-nums text-neutral-400 min-w-[4.6rem]">
            {msToHMMSSs(clip.endMs)}
          </span>

          <div className="flex-1" />

          {onUpdateTimes && (
            <div className="flex flex-wrap items-center gap-1">
              <span className="time-label">Start</span>
              <button type="button" aria-label="Start one second earlier" onClick={() => nudge('start', -1000)} className="text-sm text-white/70 hover:text-white px-1">−1s</button>
              <button type="button" aria-label="Start one second later" onClick={() => nudge('start', 1000)} className="text-sm text-white/70 hover:text-white px-1">+1s</button>
              <input
                type="text"
                value={startInput}
                onChange={(e) => setStartInput(e.target.value)}
                onBlur={handleStartBlur}
                aria-label="Start time"
                className="w-[5rem] h-8 bg-transparent px-1 text-white text-sm text-center font-mono outline-none"
              />
              <span className="time-label">End</span>
              <input
                type="text"
                value={endInput}
                onChange={(e) => setEndInput(e.target.value)}
                onBlur={handleEndBlur}
                aria-label="End time"
                className="w-[5rem] h-8 bg-transparent px-1 text-white text-sm text-center font-mono outline-none"
              />
              <button type="button" aria-label="End one second earlier" onClick={() => nudge('end', -1000)} className="text-sm text-white/70 hover:text-white px-1">−1s</button>
              <button type="button" aria-label="End one second later" onClick={() => nudge('end', 1000)} className="text-sm text-white/70 hover:text-white px-1">+1s</button>
            </div>
          )}

          <div className="flex items-center gap-1.5 min-w-[7.5rem]">
            <button
              type="button"
              onClick={() => setMuted((on) => !on)}
              className="w-8 h-8 rounded-md text-neutral-300 hover:text-white grid place-items-center"
              aria-label={muted || volume === 0 ? 'Unmute' : 'Mute'}
            >
              {muted || volume === 0 ? (
                <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
                  <path d="M2 6.5h2.2L7 4v8L4.2 9.5H2v-3z" fill="currentColor" />
                  <path d="M10 6l4 4M14 6l-4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
                </svg>
              ) : (
                <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M2 6.5h2.2L7 4v8L4.2 9.5H2v-3z" />
                  <path d="M9.2 6.2a3 3 0 010 3.6" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                  <path d="M11 4.8a5 5 0 010 6.4" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                </svg>
              )}
            </button>
            <input
              type="range"
              min={0}
              max={1}
              step={0.01}
              value={effectiveVolume}
              onChange={(e) => {
                const next = Number(e.target.value)
                setVolume(next)
                if (next > 0) setMuted(false)
              }}
              aria-label="Volume"
              className="volume-slider"
            />
          </div>

          <button
            type="button"
            onClick={toggleFullscreen}
            className="w-8 h-8 rounded-md text-neutral-300 hover:text-white grid place-items-center"
            aria-label={fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
          >
            {fullscreen ? (
              <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                <path d="M5 3v2H3M11 3v2h2M3 11h2v2M13 11h-2v2" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            ) : (
              <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
                <path d="M3 6V3h3M10 3h3v3M13 10v3h-3M6 13H3v-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              </svg>
            )}
          </button>
        </div>

        {onUpdateCrop && (
          <div className="option-row">
            {CROP_PRESETS.map((preset) => (
              <button
                key={preset.ratio}
                type="button"
                onClick={() => setRatio(preset.ratio)}
                className={`option option-sm ${crop.ratio === preset.ratio ? 'is-selected' : ''}`}
              >
                {preset.label}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
})

export default ClipCard
