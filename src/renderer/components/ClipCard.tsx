import React, { useEffect, useRef, useState } from 'react'
import VideoPreview, { type VideoPreviewHandle } from './VideoPreview'
import type { ClipSegmentWithStatus } from '../../shared/types'
import { clamp } from '../../shared/crop'
import {
  trimEdge,
  nudgeEdge,
  clipViewWindow,
  expandViewToFit,
  secondsFromDrag,
  type ClipViewWindow
} from '../../shared/clipBounds'
import { formatPlaybackClock } from '../lib/playback'
import { pointerToSourceMs } from '../../shared/previewUrl'

const MIN_CLIP_MS = 500
const HOLD_ARM_MS = 280
const HOLD_STEP_MS = 340
const EDGE_PX = 3

function formatDuration(startMs: number, endMs: number): string {
  const totalSeconds = Math.round((endMs - startMs) / 1000)
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  if (h > 0) return `${h}h ${m}m ${s}s`
  return m > 0 ? `${m}m ${s}s` : `${s}s`
}

function videoLimit(durationMs: number | undefined, startMs: number, endMs: number): number {
  return durationMs && durationMs > 0 ? durationMs : Math.max(endMs, startMs + MIN_CLIP_MS, 1)
}

interface ClipCardProps {
  clip: ClipSegmentWithStatus
  videoPath: string
  videoDurationMs?: number
  onUpdateTimes?: (id: string, startMs: number, endMs: number) => void
}

const ClipCard = React.memo(function ClipCard({
  clip,
  videoPath,
  videoDurationMs,
  onUpdateTimes
}: ClipCardProps): React.JSX.Element {
  const playerRef = useRef<VideoPreviewHandle>(null)
  const trackRef = useRef<HTMLDivElement>(null)
  const [playMs, setPlayMs] = useState(clip.startMs)
  const [seekToMs, setSeekToMs] = useState(clip.startMs)
  const [seekNonce, setSeekNonce] = useState(0)
  const [playing, setPlaying] = useState(false)
  const edgesRef = useRef({ startMs: clip.startMs, endMs: clip.endMs })
  edgesRef.current = { startMs: clip.startMs, endMs: clip.endMs }
  const limit = videoLimit(videoDurationMs, clip.startMs, clip.endMs)
  const limitRef = useRef(limit)
  limitRef.current = limit
  const [view, setView] = useState<ClipViewWindow>(() =>
    clipViewWindow(clip.startMs, clip.endMs, videoLimit(videoDurationMs, clip.startMs, clip.endMs))
  )
  const viewRef = useRef(view)
  viewRef.current = view
  const gestureCleanup = useRef<(() => void) | null>(null)
  const hadDuration = useRef(Boolean(videoDurationMs && videoDurationMs > 0))

  const inTail = playMs < clip.startMs - 40 || playMs > clip.endMs + 40

  const fitView = (startMs: number, endMs: number): void => {
    setView((current) => {
      const next = expandViewToFit(current, startMs, endMs, limitRef.current)
      if (next.viewStart === current.viewStart && next.viewEnd === current.viewEnd) return current
      return next
    })
  }

  useEffect(() => {
    setPlayMs(clip.startMs)
    setSeekToMs(clip.startMs)
    setSeekNonce((n) => n + 1)
    setView(clipViewWindow(clip.startMs, clip.endMs, limitRef.current))
    return () => {
      gestureCleanup.current?.()
    }
  }, [clip.id])

  useEffect(() => {
    const has = Boolean(videoDurationMs && videoDurationMs > 0)
    if (has && !hadDuration.current) {
      const edges = edgesRef.current
      setView(clipViewWindow(edges.startMs, edges.endMs, videoDurationMs as number))
    }
    hadDuration.current = has
  }, [videoDurationMs])

  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return
      if (e.key === ' ' || e.code === 'Space') {
        e.preventDefault()
        playerRef.current?.togglePlay()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const seek = (ms: number): void => {
    const next = clamp(ms, 0, limitRef.current)
    setPlayMs(next)
    setSeekToMs(next)
    setSeekNonce((n) => n + 1)
    playerRef.current?.seek(next)
  }

  const percent = (ms: number): number => {
    const span = view.viewEnd - view.viewStart
    if (!(span > 0)) return 0
    return clamp(((ms - view.viewStart) / span) * 100, 0, 100)
  }

  const onTimelinePointer = (e: React.PointerEvent, mode: 'in' | 'out' | 'play'): void => {
    if ((mode === 'in' || mode === 'out') && !onUpdateTimes) return
    e.preventDefault()
    e.stopPropagation()
    const edge = mode === 'in' ? 'start' : 'end'
    const frozen = { ...edgesRef.current }
    // The window stays fixed for the whole gesture, so the other handle does not slide.
    const frozenView = { ...viewRef.current }
    const scaleEnd = limitRef.current
    let live = { ...frozen }
    let holdTimer: ReturnType<typeof setTimeout> | null = null
    let stepOriginMs: number | null = null
    let steps = 0
    let lastX = e.clientX

    const publish = (next: { startMs: number; endMs: number }): void => {
      live = next
      onUpdateTimes?.(clip.id, next.startMs, next.endMs)
      seek(edge === 'start' ? next.startMs : next.endMs)
    }

    const outward = (clientX: number): boolean => {
      const track = trackRef.current
      if (!track || mode === 'play') return false
      const rect = track.getBoundingClientRect()
      const slop = steps > 0 ? 16 : EDGE_PX
      if (mode === 'in') return clientX <= rect.left + slop
      return clientX >= rect.right - slop
    }

    const clearHold = (): void => {
      if (holdTimer != null) {
        clearTimeout(holdTimer)
        holdTimer = null
      }
    }

    const endGesture = (): void => {
      clearHold()
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      if (gestureCleanup.current === endGesture) gestureCleanup.current = null
    }

    const apply = (clientX: number): void => {
      lastX = clientX
      const track = trackRef.current
      if (!track) return
      const rect = track.getBoundingClientRect()
      const ms = pointerToSourceMs(
        clientX,
        rect.left,
        rect.width,
        frozenView.viewStart,
        frozenView.viewEnd
      )
      if (mode === 'play') {
        seek(ms)
        return
      }
      if (outward(clientX)) {
        if (steps === 0) publish(trimEdge(edge, ms, frozen.startMs, frozen.endMs, scaleEnd))
        if (holdTimer == null) {
          holdTimer = setTimeout(function tick() {
            if (!outward(lastX)) {
              holdTimer = null
              return
            }
            if (stepOriginMs == null) {
              stepOriginMs = edge === 'start' ? live.startMs : live.endMs
            }
            steps += 1
            const delta = (edge === 'start' ? -1 : 1) * steps * 1000
            publish(
              nudgeEdge(
                edge === 'start' ? stepOriginMs : frozen.startMs,
                edge === 'end' ? stepOriginMs : frozen.endMs,
                edge,
                delta,
                scaleEnd
              )
            )
            holdTimer = setTimeout(tick, HOLD_STEP_MS)
          }, HOLD_ARM_MS)
        }
        return
      }
      clearHold()
      steps = 0
      stepOriginMs = null
      publish(trimEdge(edge, ms, frozen.startMs, frozen.endMs, scaleEnd))
    }

    const move = (ev: PointerEvent): void => apply(ev.clientX)
    const up = (): void => {
      endGesture()
      if (mode === 'play') return
      fitView(live.startMs, live.endMs)
    }

    gestureCleanup.current?.()
    gestureCleanup.current = endGesture
    apply(e.clientX)
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  const onTimePointer = (e: React.PointerEvent, edge: 'start' | 'end'): void => {
    if (!onUpdateTimes) return
    e.preventDefault()
    e.stopPropagation()
    const originX = e.clientX
    const origin = { ...edgesRef.current }
    let live = origin
    let applied = 0

    const endGesture = (): void => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
      if (gestureCleanup.current === endGesture) gestureCleanup.current = null
    }
    const move = (ev: PointerEvent): void => {
      const seconds = secondsFromDrag(ev.clientX - originX)
      if (seconds === applied) return
      applied = seconds
      live = nudgeEdge(origin.startMs, origin.endMs, edge, seconds * 1000, limitRef.current)
      onUpdateTimes(clip.id, live.startMs, live.endMs)
      seek(edge === 'start' ? live.startMs : live.endMs)
    }
    const up = (): void => {
      endGesture()
      fitView(live.startMs, live.endMs)
    }

    gestureCleanup.current?.()
    gestureCleanup.current = endGesture
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  const onTimeKey = (e: React.KeyboardEvent, edge: 'start' | 'end'): void => {
    if (e.key === ' ' || e.code === 'Space') {
      e.preventDefault()
      return
    }
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
    e.preventDefault()
    e.stopPropagation()
    if (!onUpdateTimes) return
    const delta = e.key === 'ArrowLeft' ? -1000 : 1000
    const next = nudgeEdge(
      edgesRef.current.startMs,
      edgesRef.current.endMs,
      edge,
      delta,
      limitRef.current
    )
    onUpdateTimes(clip.id, next.startMs, next.endMs)
    seek(edge === 'start' ? next.startMs : next.endMs)
    fitView(next.startMs, next.endMs)
  }

  const x0 = percent(clip.startMs)
  const x1 = percent(clip.endMs)
  const xp = percent(playMs)
  const startClock = formatPlaybackClock(clip.startMs)
  const endClock = formatPlaybackClock(clip.endMs)

  return (
    <div className="flex flex-col flex-1 min-h-0 bg-black">
      <div className="relative flex-1 min-h-0">
        <VideoPreview
          ref={playerRef}
          videoPath={videoPath}
          startMs={clip.startMs}
          endMs={clip.endMs}
          videoDurationMs={videoDurationMs}
          onPlayheadMs={setPlayMs}
          onPlayingChange={setPlaying}
          seekToMs={seekToMs}
          seekNonce={seekNonce}
          editor
          transport={false}
        />
        <div className="pointer-events-none absolute inset-x-0 top-0 z-10 bg-gradient-to-b from-black/55 to-transparent px-5 pt-4 pb-10">
          <h3 className="text-lg font-medium text-white truncate">{clip.title}</h3>
          <p className="text-sm text-white/70 truncate">{formatDuration(clip.startMs, clip.endMs)}</p>
        </div>
        {inTail && (
          <div className="absolute right-4 top-3 z-20 text-sm text-white/80">Outside the clip</div>
        )}
      </div>

      <div className="shrink-0 bg-bg-base border-t border-white/8 px-4 py-2 flex items-center gap-3">
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
          <span className="text-sm font-mono tabular-nums text-neutral-200 shrink-0 min-w-[3.25rem]" aria-label="Current time">
            {formatPlaybackClock(playMs)}
          </span>
          {onUpdateTimes && (
            <button
              type="button"
              data-clip-time="start"
              aria-label={`Start time ${startClock}`}
              className="cursor-ew-resize select-none touch-none shrink-0 whitespace-nowrap text-sm font-mono tabular-nums text-neutral-300 hover:text-white"
              onPointerDown={(e) => onTimePointer(e, 'start')}
              onKeyDown={(e) => onTimeKey(e, 'start')}
            >
              Start {startClock}
            </button>
          )}
          <div
            className="relative h-7 min-w-0 flex-1 cursor-pointer select-none touch-none"
            onPointerDown={(e) => onTimelinePointer(e, 'play')}
          >
            <div
              ref={trackRef}
              className="absolute left-0 right-0 w-full top-1/2 h-1 -mt-0.5 bg-white/15 rounded-full"
            >
              <div
                className="absolute top-0 h-full bg-accent rounded-full"
                style={{ left: `${x0}%`, width: `${Math.max(0.4, x1 - x0)}%` }}
              />
              {onUpdateTimes && (
                <>
                  <button
                    type="button"
                    aria-label="Start"
                    className="absolute top-1/2 z-[2] h-6 w-4 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize"
                    style={{ left: `${x0}%` }}
                    onPointerDown={(e) => onTimelinePointer(e, 'in')}
                  >
                    <span className="mx-auto block h-5 w-1 rounded-sm bg-accent" />
                  </button>
                  <button
                    type="button"
                    aria-label="End"
                    className="absolute top-1/2 z-[2] h-6 w-4 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize"
                    style={{ left: `${x1}%` }}
                    onPointerDown={(e) => onTimelinePointer(e, 'out')}
                  >
                    <span className="mx-auto block h-5 w-1 rounded-sm bg-accent" />
                  </button>
                </>
              )}
              <div
                className="absolute top-[-8px] bottom-[-8px] w-px bg-white z-[3] pointer-events-none"
                style={{ left: `${xp}%` }}
              />
            </div>
          </div>
          {onUpdateTimes && (
            <button
              type="button"
              data-clip-time="end"
              aria-label={`End time ${endClock}`}
              className="cursor-ew-resize select-none touch-none shrink-0 whitespace-nowrap text-sm font-mono tabular-nums text-neutral-300 hover:text-white"
              onPointerDown={(e) => onTimePointer(e, 'end')}
              onKeyDown={(e) => onTimeKey(e, 'end')}
            >
              End {endClock}
            </button>
          )}
      </div>
    </div>
  )
})

export default ClipCard
