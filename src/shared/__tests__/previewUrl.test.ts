import { describe, it, expect } from 'vitest'
import {
  toPreviewSrc,
  pathFromPreviewUrl,
  paddedPreviewRange,
  previewCoversRange,
  previewLocalTime,
  playbackTimeOnPlay,
  pointerToSourceMs,
  shouldPublishPlayhead
} from '../previewUrl'

describe('previewUrl', () => {
  it('round-trips a Windows path', () => {
    const filePath = 'C:\\Users\\water\\Videos\\sermon.mp4'
    const src = toPreviewSrc(filePath)
    expect(src.startsWith('a1slice://preview/')).toBe(true)
    expect(src.includes('?')).toBe(false)
    expect(pathFromPreviewUrl(src)).toBe('C:/Users/water/Videos/sermon.mp4')
  })

  it('still reads the legacy query-string form', () => {
    const src = 'a1slice://video?path=C%3A%2FUsers%2Fwater%2Fclip.mov'
    expect(pathFromPreviewUrl(src)).toBe('C:/Users/water/clip.mov')
  })

  it('pads a clip window so small time nudges stay inside the preview file', () => {
    const range = paddedPreviewRange(60_000, 90_000, 600_000)
    expect(range.fileStartMs).toBe(48_000)
    expect(range.fileEndMs).toBe(102_000)
    expect(previewCoversRange(range, 59_000, 91_000)).toBe(true)
    expect(previewCoversRange(range, 40_000, 90_000)).toBe(false)
  })
})

describe('previewLocalTime', () => {
  it('maps a source timestamp onto the preview file timeline', () => {
    expect(previewLocalTime(57_600, 45_600)).toBe(12)
  })

  it('does not go below zero', () => {
    expect(previewLocalTime(1_000, 5_000)).toBe(0)
  })
})

describe('playbackTimeOnPlay', () => {
  const inPoint = 12
  const outPoint = 58.3

  it('starts at the clip in-point when the video is still at t=0', () => {
    expect(playbackTimeOnPlay(0, inPoint, outPoint)).toBe(inPoint)
  })

  it('starts at the clip in-point when a prior seek to t=0 was lost', () => {
    expect(playbackTimeOnPlay(0.02, inPoint, outPoint)).toBe(inPoint)
  })

  it('keeps the playhead when it is already inside the clip', () => {
    expect(playbackTimeOnPlay(20, inPoint, outPoint)).toBe(20)
  })

  it('keeps a playhead the user seeked into pre-roll', () => {
    expect(playbackTimeOnPlay(5, inPoint, outPoint)).toBe(5)
  })

  it('restarts at the in-point when play is pressed at or past the out-point', () => {
    expect(playbackTimeOnPlay(outPoint, inPoint, outPoint)).toBe(inPoint)
    expect(playbackTimeOnPlay(outPoint + 0.4, inPoint, outPoint)).toBe(inPoint)
  })

  it('plays from zero when the clip itself starts at the file start', () => {
    expect(playbackTimeOnPlay(0, 0, 10)).toBe(0)
  })
})

describe('pointerToSourceMs', () => {
  const srcStart = 45_600
  const srcEnd = 115_900
  const left = 100
  const width = 400

  it('maps the left edge of the bar to the preview start', () => {
    expect(pointerToSourceMs(left, left, width, srcStart, srcEnd)).toBe(srcStart)
  })

  it('maps the right edge of the bar to the preview end', () => {
    expect(pointerToSourceMs(left + width, left, width, srcStart, srcEnd)).toBe(srcEnd)
  })

  it('maps the middle of the bar to the midpoint, not zero', () => {
    expect(pointerToSourceMs(left + width / 2, left, width, srcStart, srcEnd)).toBe(
      (srcStart + srcEnd) / 2
    )
  })

  it('clamps clicks past the ends instead of wrapping to zero', () => {
    expect(pointerToSourceMs(left - 80, left, width, srcStart, srcEnd)).toBe(srcStart)
    expect(pointerToSourceMs(left + width + 80, left, width, srcStart, srcEnd)).toBe(srcEnd)
  })

  it('does not return NaN when the track has no width', () => {
    expect(pointerToSourceMs(140, left, 0, srcStart, srcEnd)).toBe(srcStart)
    expect(pointerToSourceMs(140, left, Number.NaN, srcStart, srcEnd)).toBe(srcStart)
  })
})

describe('shouldPublishPlayhead', () => {
  it('does not publish while the element is seeking or still waiting for the in-point', () => {
    expect(shouldPublishPlayhead(true, false)).toBe(false)
    expect(shouldPublishPlayhead(false, true)).toBe(false)
  })

  it('publishes once the playhead is real', () => {
    expect(shouldPublishPlayhead(false, false)).toBe(true)
  })
})
