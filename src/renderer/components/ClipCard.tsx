import { useState, useEffect } from 'react'
import VideoPreview from './VideoPreview'
import type { ClipSegmentWithStatus } from '../../shared/types'

function formatDuration(startMs: number, endMs: number): string {
  const totalSeconds = Math.round((endMs - startMs) / 1000)
  const m = Math.floor(totalSeconds / 60)
  const s = totalSeconds % 60
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

function msToMMSSs(ms: number): string {
  const totalSeconds = ms / 1000
  const m = Math.floor(totalSeconds / 60)
  const s = totalSeconds % 60
  return `${String(m).padStart(2, '0')}:${s.toFixed(1).padStart(4, '0')}`
}

function parseMMSSs(value: string): number | null {
  const match = value.trim().match(/^(\d+):(\d+(?:\.\d+)?)$/)
  if (!match) return null
  const m = parseInt(match[1], 10)
  const s = parseFloat(match[2])
  if (isNaN(m) || isNaN(s) || s >= 60) return null
  const ms = (m * 60 + s) * 1000
  return ms >= 0 ? Math.round(ms) : null
}

interface ClipCardProps {
  clip: ClipSegmentWithStatus
  videoPath: string
  videoDurationMs?: number
  onToggle: (id: string) => void
  onUpdateTimes?: (id: string, startMs: number, endMs: number) => void
}

export default function ClipCard({
  clip,
  videoPath,
  videoDurationMs,
  onToggle,
  onUpdateTimes
}: ClipCardProps): React.JSX.Element {
  const [startInput, setStartInput] = useState(msToMMSSs(clip.startMs))
  const [endInput, setEndInput] = useState(msToMMSSs(clip.endMs))

  useEffect(() => {
    setStartInput(msToMMSSs(clip.startMs))
  }, [clip.startMs])

  useEffect(() => {
    setEndInput(msToMMSSs(clip.endMs))
  }, [clip.endMs])

  const handleStartBlur = (): void => {
    const ms = parseMMSSs(startInput)
    const maxMs = videoDurationMs ?? Infinity
    if (ms !== null && ms < clip.endMs && ms <= maxMs && onUpdateTimes) {
      onUpdateTimes(clip.id, ms, clip.endMs)
    } else {
      setStartInput(msToMMSSs(clip.startMs))
    }
  }

  const handleEndBlur = (): void => {
    const ms = parseMMSSs(endInput)
    const maxMs = videoDurationMs ?? Infinity
    if (ms !== null && ms > clip.startMs && ms <= maxMs && onUpdateTimes) {
      onUpdateTimes(clip.id, clip.startMs, ms)
    } else {
      setEndInput(msToMMSSs(clip.endMs))
    }
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
            <h3 className="text-sm font-medium text-neutral-200">
              {clip.title}
            </h3>
            <p className="text-xs text-neutral-400 font-mono mt-1">
              {msToMMSSs(clip.startMs)} — {msToMMSSs(clip.endMs)}
              <span className="text-neutral-500 ml-2">
                ({formatDuration(clip.startMs, clip.endMs)})
              </span>
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
        {onUpdateTimes && (
          <div className="flex items-center gap-2 text-xs">
            <input
              type="text"
              value={startInput}
              onChange={(e) => setStartInput(e.target.value)}
              onBlur={handleStartBlur}
              className="w-20 bg-bg-input border border-white/12 rounded px-2 py-1 text-neutral-300 text-center font-mono"
            />
            <span className="text-neutral-500">—</span>
            <input
              type="text"
              value={endInput}
              onChange={(e) => setEndInput(e.target.value)}
              onBlur={handleEndBlur}
              className="w-20 bg-bg-input border border-white/12 rounded px-2 py-1 text-neutral-300 text-center font-mono"
            />
          </div>
        )}
      </div>
    </div>
  )
}
