import React, { useState, useEffect } from 'react'
import VideoPreview from './VideoPreview'
import type { ClipSegmentWithStatus } from '../../shared/types'

const MIN_CLIP_MS = 500
const NUDGE_MS = 1000

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
  // Try H:MM:SS.S (two colons)
  const hMatch = value.trim().match(/^(\d+):(\d+):(\d+(?:\.\d+)?)$/)
  if (hMatch) {
    const h = parseInt(hMatch[1], 10)
    const m = parseInt(hMatch[2], 10)
    const s = parseFloat(hMatch[3])
    if (isNaN(h) || isNaN(m) || isNaN(s) || m >= 60 || s >= 60) return null
    const ms = (h * 3600 + m * 60 + s) * 1000
    return ms >= 0 ? Math.round(ms) : null
  }
  // Try MM:SS.S (one colon)
  const mMatch = value.trim().match(/^(\d+):(\d+(?:\.\d+)?)$/)
  if (!mMatch) return null
  const m = parseInt(mMatch[1], 10)
  const s = parseFloat(mMatch[2])
  if (isNaN(m) || isNaN(s) || s >= 60) return null
  const ms = (m * 60 + s) * 1000
  return ms >= 0 ? Math.round(ms) : null
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

interface ClipCardProps {
  clip: ClipSegmentWithStatus
  videoPath: string
  videoDurationMs?: number
  onToggle: (id: string) => void
  onUpdateTimes?: (id: string, startMs: number, endMs: number) => void
}

const ClipCard = React.memo(function ClipCard({
  clip,
  videoPath,
  videoDurationMs,
  onToggle,
  onUpdateTimes
}: ClipCardProps): React.JSX.Element {
  const [startInput, setStartInput] = useState(msToHMMSSs(clip.startMs))
  const [endInput, setEndInput] = useState(msToHMMSSs(clip.endMs))
  const maxMs = videoDurationMs && videoDurationMs > 0
    ? videoDurationMs
    : Math.max(clip.endMs + 120000, clip.startMs + 120000)
  const minGapMs = Math.max(50, Math.min(MIN_CLIP_MS, clip.endMs - clip.startMs))

  useEffect(() => {
    setStartInput(msToHMMSSs(clip.startMs))
  }, [clip.startMs])

  useEffect(() => {
    setEndInput(msToHMMSSs(clip.endMs))
  }, [clip.endMs])

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

  const nudgeStart = (deltaMs: number): void => {
    if (!onUpdateTimes) return
    const maxStart = Math.min(maxMs, clip.endMs - minGapMs)
    onUpdateTimes(clip.id, clamp(clip.startMs + deltaMs, 0, maxStart), clip.endMs)
  }

  const nudgeEnd = (deltaMs: number): void => {
    if (!onUpdateTimes) return
    const minEnd = clip.startMs + minGapMs
    onUpdateTimes(clip.id, clip.startMs, clamp(clip.endMs + deltaMs, minEnd, maxMs))
  }

  return (
    <div
      className={`flex flex-row bg-bg-card border rounded-2xl overflow-hidden transition-colors ${
        clip.approved ? 'border-accent/30' : 'border-white/7 opacity-60'
      }`}
    >
      <div className="w-[280px] shrink-0">
        <VideoPreview
          videoPath={videoPath}
          startMs={clip.startMs}
          endMs={clip.endMs}
        />
      </div>
      <div className="flex-1 p-4 flex flex-col justify-center gap-2 min-w-0">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h3 className="text-lg font-semibold text-neutral-200">
              {clip.title}
              <span className="text-sm font-normal text-neutral-500 ml-2">
                {formatDuration(clip.startMs, clip.endMs)}
              </span>
            </h3>
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
        {onUpdateTimes && (
          <div className="flex items-center gap-2 text-xs flex-wrap">
            <div className="flex items-center gap-1">
              <button
                onClick={() => nudgeStart(-NUDGE_MS)}
                className="h-6 px-1.5 rounded border border-white/15 bg-bg-input text-neutral-400 text-[11px] hover:border-accent/40 hover:text-neutral-200"
              >
                -1s
              </button>
              <input
                type="text"
                value={startInput}
                onChange={(e) => setStartInput(e.target.value)}
                onBlur={handleStartBlur}
                className="w-[5.5rem] bg-bg-input border border-white/12 rounded px-2 py-1 text-neutral-300 text-center font-mono"
              />
              <button
                onClick={() => nudgeStart(NUDGE_MS)}
                className="h-6 px-1.5 rounded border border-white/15 bg-bg-input text-neutral-400 text-[11px] hover:border-accent/40 hover:text-neutral-200"
              >
                +1s
              </button>
            </div>
            <span className="text-neutral-500">—</span>
            <div className="flex items-center gap-1">
              <button
                onClick={() => nudgeEnd(-NUDGE_MS)}
                className="h-6 px-1.5 rounded border border-white/15 bg-bg-input text-neutral-400 text-[11px] hover:border-accent/40 hover:text-neutral-200"
              >
                -1s
              </button>
              <input
                type="text"
                value={endInput}
                onChange={(e) => setEndInput(e.target.value)}
                onBlur={handleEndBlur}
                className="w-[5.5rem] bg-bg-input border border-white/12 rounded px-2 py-1 text-neutral-300 text-center font-mono"
              />
              <button
                onClick={() => nudgeEnd(NUDGE_MS)}
                className="h-6 px-1.5 rounded border border-white/15 bg-bg-input text-neutral-400 text-[11px] hover:border-accent/40 hover:text-neutral-200"
              >
                +1s
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
})

export default ClipCard
