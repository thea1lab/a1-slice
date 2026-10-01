import type { ClipSegment, TranscriptSegment } from './types'
import { clamp } from './crop'

const PAUSE_MIN_MS = 400
const PRE_ROLL_MS = 250
const POST_ROLL_MS = 250
const PAUSE_EDGE_PAD_MS = 40

export function swapInvertedTimes<T extends { startMs: number; endMs: number }>(clip: T): T {
  if (clip.startMs <= clip.endMs) return clip
  return { ...clip, startMs: clip.endMs, endMs: clip.startMs }
}

function findSegmentIndexAt(
  ms: number,
  segments: TranscriptSegment[],
  prefer: 'start' | 'end'
): number {
  if (segments.length === 0) return -1
  for (let i = 0; i < segments.length; i++) {
    const s = segments[i]
    if (ms >= s.startMs && ms <= s.endMs) return i
  }
  if (prefer === 'start') {
    for (let i = 0; i < segments.length; i++) {
      if (segments[i].startMs >= ms) return i
    }
    return segments.length - 1
  }
  for (let i = segments.length - 1; i >= 0; i--) {
    if (segments[i].endMs <= ms) return i
  }
  return 0
}

export function snapClipToSegments<T extends { startMs: number; endMs: number }>(
  clip: T,
  segments: TranscriptSegment[]
): T {
  if (segments.length === 0) return clip
  const startIdx = findSegmentIndexAt(clip.startMs, segments, 'start')
  const endIdx = findSegmentIndexAt(clip.endMs, segments, 'end')
  const startMs = segments[startIdx].startMs
  const endMs = segments[endIdx].endMs
  if (endMs <= startMs) {
    return { ...clip, startMs, endMs: Math.max(endMs, segments[startIdx].endMs) }
  }
  return { ...clip, startMs, endMs }
}

export function snapToPause<T extends { startMs: number; endMs: number }>(
  clip: T,
  segments: TranscriptSegment[]
): T {
  if (segments.length === 0) return clip

  const startIdxExact = segments.findIndex((s) => s.startMs === clip.startMs)
  const endIdxExact = segments.findIndex((s) => s.endMs === clip.endMs)
  const startIdx = startIdxExact >= 0 ? startIdxExact : findSegmentIndexAt(clip.startMs, segments, 'start')
  const endIdx = endIdxExact >= 0 ? endIdxExact : findSegmentIndexAt(clip.endMs, segments, 'end')

  let startMs = clip.startMs
  let endMs = clip.endMs

  if (startIdx > 0) {
    const prev = segments[startIdx - 1]
    const cur = segments[startIdx]
    const gap = cur.startMs - prev.endMs
    if (gap >= PAUSE_MIN_MS) {
      startMs = Math.max(prev.endMs + PAUSE_EDGE_PAD_MS, cur.startMs - PRE_ROLL_MS)
    }
  }

  if (endIdx >= 0 && endIdx < segments.length - 1) {
    const cur = segments[endIdx]
    const next = segments[endIdx + 1]
    const gap = next.startMs - cur.endMs
    if (gap >= PAUSE_MIN_MS) {
      endMs = Math.min(next.startMs - PAUSE_EDGE_PAD_MS, cur.endMs + POST_ROLL_MS)
    }
  }

  return { ...clip, startMs, endMs }
}

/** Move one edge. The other time stays where it was. */
export function trimEdge(
  edge: 'start' | 'end',
  pointerMs: number,
  startMs: number,
  endMs: number,
  maxMs: number,
  minLen = 500
): { startMs: number; endMs: number } {
  const limit = Number.isFinite(maxMs) && maxMs > 0 ? maxMs : Math.max(endMs, startMs)
  if (edge === 'start') {
    const start = clamp(pointerMs, 0, Math.max(0, endMs - minLen))
    return { startMs: Math.round(start), endMs }
  }
  const end = clamp(pointerMs, Math.min(limit, startMs + minLen), limit)
  return { startMs, endMs: Math.round(end) }
}

/** One minute of video on each side of a clip, shown on the review bar. */
export const CLIP_VIEW_PAD_MS = 60_000

/** Dragging a start or end time this many pixels moves that edge by one second. */
export const FINE_DRAG_PX = 24

export interface ClipViewWindow {
  viewStart: number
  viewEnd: number
}

function videoLimit(maxMs: number, startMs: number, endMs: number): number {
  return Number.isFinite(maxMs) && maxMs > 0 ? maxMs : Math.max(endMs, startMs, 1)
}

/** The bar shows the clip plus one minute before and after, clamped to the video. */
export function clipViewWindow(
  startMs: number,
  endMs: number,
  maxMs: number,
  padMs = CLIP_VIEW_PAD_MS
): ClipViewWindow {
  const limit = videoLimit(maxMs, startMs, endMs)
  const start = clamp(startMs, 0, limit)
  const end = clamp(Math.max(endMs, start), 0, limit)
  const viewStart = Math.max(0, start - padMs)
  let viewEnd = Math.min(limit, end + padMs)
  if (viewEnd - viewStart < 1) viewEnd = Math.min(limit, viewStart + 1)
  return { viewStart: Math.round(viewStart), viewEnd: Math.round(viewEnd) }
}

/**
 * Grow the window so an edge that moved past it stays visible, with the same pad.
 * The side that did not move stays put.
 */
export function expandViewToFit(
  view: ClipViewWindow,
  startMs: number,
  endMs: number,
  maxMs: number,
  padMs = CLIP_VIEW_PAD_MS
): ClipViewWindow {
  const limit = videoLimit(maxMs, startMs, Math.max(endMs, view.viewEnd))
  let viewStart = view.viewStart
  let viewEnd = view.viewEnd
  if (startMs < viewStart) viewStart = Math.max(0, startMs - padMs)
  if (endMs > viewEnd) viewEnd = Math.min(limit, endMs + padMs)
  if (viewEnd - viewStart < 1) viewEnd = Math.min(limit, viewStart + 1)
  return { viewStart: Math.round(viewStart), viewEnd: Math.round(viewEnd) }
}

/** Whole seconds from a horizontal drag. Right is later, left is earlier. */
export function secondsFromDrag(dxPx: number, pxPerSecond = FINE_DRAG_PX): number {
  if (!Number.isFinite(dxPx) || !(pxPerSecond > 0)) return 0
  return Math.trunc(dxPx / pxPerSecond)
}

export function nudgeEdge(
  startMs: number,
  endMs: number,
  edge: 'start' | 'end',
  deltaMs: number,
  maxMs: number,
  minLen = 500
): { startMs: number; endMs: number } {
  const limit = Number.isFinite(maxMs) && maxMs > 0 ? maxMs : Math.max(endMs, startMs)
  let start = startMs + (edge === 'start' ? deltaMs : 0)
  let end = endMs + (edge === 'end' ? deltaMs : 0)
  start = clamp(start, 0, limit)
  end = clamp(end, 0, limit)
  if (end - start < minLen) {
    if (edge === 'start') start = clamp(end - minLen, 0, limit)
    else end = clamp(start + minLen, 0, limit)
  }
  if (end - start < minLen) {
    if (edge === 'start') end = clamp(start + minLen, 0, limit)
    else start = clamp(end - minLen, 0, limit)
  }
  return { startMs: Math.round(start), endMs: Math.round(end) }
}

export function refineClipBounds<T extends ClipSegment>(
  clip: T,
  segments: TranscriptSegment[]
): T {
  let result = swapInvertedTimes(clip)
  if (segments.length === 0) {
    return {
      ...result,
      startMs: Math.max(0, result.startMs),
      endMs: Math.max(0, result.endMs)
    }
  }

  const maxMs = segments[segments.length - 1].endMs
  result = {
    ...result,
    startMs: Math.max(0, Math.min(result.startMs, maxMs)),
    endMs: Math.max(0, Math.min(result.endMs, maxMs))
  }
  result = swapInvertedTimes(result)
  result = snapClipToSegments(result, segments)
  result = snapToPause(result, segments)
  return result
}
