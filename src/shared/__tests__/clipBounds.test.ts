import { describe, it, expect } from 'vitest'
import {
  swapInvertedTimes,
  snapClipToSegments,
  snapToPause,
  nudgeEdge,
  refineClipBounds,
  trimEdge,
  clipViewWindow,
  expandViewToFit,
  secondsFromDrag
} from '../clipBounds'
import type { TranscriptSegment, ClipSegment } from '../types'

const segs = (pairs: [number, number, string?][]): TranscriptSegment[] =>
  pairs.map(([startMs, endMs, text], i) => ({
    startMs,
    endMs,
    text: text ?? `S${i}`
  }))

describe('swapInvertedTimes', () => {
  it('swaps when start is after end', () => {
    expect(swapInvertedTimes({ title: 'A', startMs: 9000, endMs: 1000 })).toEqual({
      title: 'A',
      startMs: 1000,
      endMs: 9000
    })
  })

  it('leaves ordered clips unchanged', () => {
    const clip: ClipSegment = { title: 'A', startMs: 0, endMs: 5000 }
    expect(swapInvertedTimes(clip)).toEqual(clip)
  })
})

describe('snapClipToSegments', () => {
  const segments = segs([
    [0, 4000],
    [4500, 9000],
    [9200, 15000],
    [16000, 22000]
  ])

  it('snaps start to containing segment start and end to containing segment end', () => {
    const snapped = snapClipToSegments(
      { title: 'A', startMs: 1200, endMs: 14000 },
      segments
    )
    expect(snapped.startMs).toBe(0)
    expect(snapped.endMs).toBe(15000)
  })

  it('snaps a time in a gap to the next segment start', () => {
    const snapped = snapClipToSegments(
      { title: 'A', startMs: 4100, endMs: 15500 },
      segments
    )
    expect(snapped.startMs).toBe(4500)
    expect(snapped.endMs).toBe(15000)
  })

  it('returns the clip unchanged when there are no segments', () => {
    const clip = { title: 'A', startMs: 100, endMs: 200 }
    expect(snapClipToSegments(clip, [])).toEqual(clip)
  })
})

describe('snapToPause', () => {
  const segments = segs([
    [0, 4000],
    [4800, 9000], // 800ms gap after previous
    [9200, 15000], // 200ms gap — too short to treat as pause
    [17000, 22000] // 2000ms gap
  ])

  it('adds a short pre-roll into a pause before the start segment', () => {
    const clip = { title: 'A', startMs: 4800, endMs: 9000 }
    const result = snapToPause(clip, segments)
    expect(result.startMs).toBeLessThan(4800)
    expect(result.startMs).toBeGreaterThanOrEqual(4000)
    expect(result.endMs).toBe(9000)
  })

  it('does not eat previous speech when the gap is tiny', () => {
    const clip = { title: 'A', startMs: 9200, endMs: 15000 }
    const result = snapToPause(clip, segments)
    expect(result.startMs).toBe(9200)
  })

  it('extends the end slightly into a following pause', () => {
    const clip = { title: 'A', startMs: 9200, endMs: 15000 }
    const result = snapToPause(clip, segments)
    expect(result.endMs).toBeGreaterThan(15000)
    expect(result.endMs).toBeLessThan(17000)
  })
})

describe('trimEdge', () => {
  it('moves the end and leaves the start where it is', () => {
    expect(trimEdge('end', 9000, 1000, 4000, 20000)).toEqual({ startMs: 1000, endMs: 9000 })
  })

  it('moves the start and leaves the end where it is', () => {
    expect(trimEdge('start', 500, 1000, 4000, 20000)).toEqual({ startMs: 500, endMs: 4000 })
  })

  it('keeps at least half a second of clip', () => {
    expect(trimEdge('end', 1000, 1000, 4000, 20000)).toEqual({ startMs: 1000, endMs: 1500 })
    expect(trimEdge('start', 9000, 1000, 4000, 20000)).toEqual({ startMs: 3500, endMs: 4000 })
  })
})

describe('clipViewWindow', () => {
  it('frames a short clip with one minute on each side', () => {
    const hour = 3_600_000
    const view = clipViewWindow(hour, hour + 21_000, 3 * hour)
    expect(view).toEqual({
      viewStart: hour - 60_000,
      viewEnd: hour + 21_000 + 60_000
    })
  })

  it('clamps the window to the start of the video', () => {
    expect(clipViewWindow(5_000, 26_000, 600_000)).toEqual({
      viewStart: 0,
      viewEnd: 86_000
    })
  })

  it('frames a clip that begins near the start of a ten minute video', () => {
    expect(clipViewWindow(10_000, 40_000, 600_000)).toEqual({
      viewStart: 0,
      viewEnd: 100_000
    })
  })
})

describe('expandViewToFit', () => {
  it('grows only the side that moved past the window', () => {
    const view = { viewStart: 1_000_000, viewEnd: 1_200_000 }
    expect(expandViewToFit(view, 1_050_000, 1_250_000, 3_600_000)).toEqual({
      viewStart: 1_000_000,
      viewEnd: 1_250_000 + 60_000
    })
  })

  it('leaves the window alone while the clip stays inside it', () => {
    const view = { viewStart: 0, viewEnd: 180_000 }
    expect(expandViewToFit(view, 10_000, 40_000, 600_000)).toEqual(view)
  })
})

describe('secondsFromDrag', () => {
  it('counts one second every 24 pixels', () => {
    expect(secondsFromDrag(23)).toBe(0)
    expect(secondsFromDrag(24)).toBe(1)
    expect(secondsFromDrag(-24)).toBe(-1)
    expect(secondsFromDrag(-47)).toBe(-1)
    expect(secondsFromDrag(48)).toBe(2)
  })
})

describe('nudgeEdge', () => {
  it('moves the start one second earlier and clamps at zero', () => {
    expect(nudgeEdge(500, 4000, 'start', -1000, 20000)).toEqual({ startMs: 0, endMs: 4000 })
  })

  it('moves the end one second later and keeps a minimum length', () => {
    expect(nudgeEdge(0, 800, 'end', -1000, 20000)).toEqual({ startMs: 0, endMs: 500 })
  })

  it('does not run past the video', () => {
    expect(nudgeEdge(1000, 2500, 'end', 1000, 3000)).toEqual({ startMs: 1000, endMs: 3000 })
  })
})

describe('refineClipBounds', () => {
  const segments = segs([
    [0, 4000, 'Hello'],
    [4500, 9000, 'World'],
    [9200, 15000, 'Punchline'],
    [16000, 22000, 'After']
  ])

  it('swaps inverted times then snaps', () => {
    const result = refineClipBounds(
      { title: 'A', startMs: 14000, endMs: 1200 },
      segments
    )
    expect(result.startMs).toBeLessThan(result.endMs)
    expect(result.startMs).toBeLessThanOrEqual(4000)
    expect(result.endMs).toBeGreaterThanOrEqual(15000)
  })

  it('clamps to the transcript range', () => {
    const result = refineClipBounds(
      { title: 'A', startMs: -500, endMs: 999999 },
      segments
    )
    expect(result.startMs).toBeGreaterThanOrEqual(0)
    expect(result.endMs).toBeLessThanOrEqual(22000)
  })

  it('preserves title and optional fields', () => {
    const result = refineClipBounds(
      { title: 'Keep', startMs: 5000, endMs: 8000, category: 'standalone', topic: 'T' },
      segments
    )
    expect(result.title).toBe('Keep')
    expect(result.category).toBe('standalone')
    expect(result.topic).toBe('T')
  })
})
