import { useRef } from 'react'
import { formatPlaybackClock, playbackMsAt } from '../lib/playback'

interface PlaybackBarProps {
  startMs: number
  endMs: number
  playheadMs: number
  playing: boolean
  onTogglePlay: () => void
  onSeek: (ms: number) => void
  onScrubbingChange?: (active: boolean) => void
}

export default function PlaybackBar({
  startMs,
  endMs,
  playheadMs,
  playing,
  onTogglePlay,
  onSeek,
  onScrubbingChange
}: PlaybackBarProps): React.JSX.Element {
  const trackRef = useRef<HTMLDivElement>(null)
  const span = Math.max(1, endMs - startMs)
  const ratio = Math.min(1, Math.max(0, (playheadMs - startMs) / span))

  const seekAt = (clientX: number): void => {
    const track = trackRef.current
    if (!track) return
    const rect = track.getBoundingClientRect()
    onSeek(playbackMsAt(clientX, rect.left, rect.width, startMs, endMs))
  }

  const nudge = (deltaMs: number): void => {
    const next = Math.min(endMs, Math.max(startMs, playheadMs + deltaMs))
    onSeek(next)
  }

  const endScrub = (): void => {
    onScrubbingChange?.(false)
  }

  return (
    <div className="flex shrink-0 items-center gap-3 border-t border-white/10 bg-black px-3 py-2">
      <button
        type="button"
        onClick={(event) => {
          event.preventDefault()
          event.stopPropagation()
          onTogglePlay()
        }}
        className="grid h-8 w-8 shrink-0 place-items-center text-white hover:text-accent"
        aria-label={playing ? 'Pause' : 'Play'}
      >
        {playing ? <PauseIcon /> : <PlayIcon />}
      </button>
      <span className="min-w-[3.25rem] text-sm tabular-nums text-[#f4f1ea]" aria-label="Current time">
        {formatPlaybackClock(playheadMs)}
      </span>
      <div
        ref={trackRef}
        role="slider"
        aria-label="Video time"
        aria-valuemin={Math.round(startMs)}
        aria-valuemax={Math.round(endMs)}
        aria-valuenow={Math.round(Math.min(endMs, Math.max(startMs, playheadMs)))}
        aria-valuetext={formatPlaybackClock(playheadMs)}
        tabIndex={0}
        className="relative h-6 min-w-0 flex-1 cursor-pointer touch-none"
        onPointerDown={(event) => {
          event.preventDefault()
          event.stopPropagation()
          event.currentTarget.setPointerCapture(event.pointerId)
          onScrubbingChange?.(true)
          seekAt(event.clientX)
        }}
        onPointerMove={(event) => {
          if (!event.currentTarget.hasPointerCapture(event.pointerId)) return
          seekAt(event.clientX)
        }}
        onPointerUp={endScrub}
        onPointerCancel={endScrub}
        onKeyDown={(event) => {
          if (event.key === 'ArrowRight') {
            event.preventDefault()
            nudge(5000)
          } else if (event.key === 'ArrowLeft') {
            event.preventDefault()
            nudge(-5000)
          } else if (event.key === 'Home') {
            event.preventDefault()
            onSeek(startMs)
          } else if (event.key === 'End') {
            event.preventDefault()
            onSeek(endMs)
          }
        }}
      >
        <div className="absolute top-1/2 right-0 left-0 h-1 -translate-y-1/2 rounded-full bg-white/20">
          <div className="h-full rounded-full bg-accent" style={{ width: `${ratio * 100}%` }} />
        </div>
        <div
          className="absolute top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white"
          style={{ left: `${ratio * 100}%` }}
        />
      </div>
      <span className="min-w-[3.25rem] text-right text-sm tabular-nums text-[#a8a8a8]" aria-label="End time">
        {formatPlaybackClock(endMs)}
      </span>
    </div>
  )
}

function PlayIcon(): React.JSX.Element {
  return (
    <svg width="14" height="14" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true">
      <path d="M3 1.5v9l7.5-4.5L3 1.5z" />
    </svg>
  )
}

function PauseIcon(): React.JSX.Element {
  return (
    <svg width="14" height="14" viewBox="0 0 12 12" fill="currentColor" aria-hidden="true">
      <rect x="2" y="1.5" width="2.5" height="9" rx="0.5" />
      <rect x="7.5" y="1.5" width="2.5" height="9" rx="0.5" />
    </svg>
  )
}
