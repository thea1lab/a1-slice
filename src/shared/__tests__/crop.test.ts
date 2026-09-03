import { describe, it, expect } from 'vitest'
import {
  cropRect,
  snapCropCenter,
  ffmpegCropFilter,
  cropFromPrevious,
  containRect,
  even
} from '../crop'
import { DEFAULT_CROP } from '../types'

describe('even', () => {
  it('rounds down to an even pixel size of at least 2', () => {
    expect(even(1080)).toBe(1080)
    expect(even(1081)).toBe(1080)
    expect(even(1)).toBe(2)
  })
})

describe('cropRect', () => {
  it('returns the full frame for original', () => {
    expect(cropRect(DEFAULT_CROP, 1920, 1080)).toEqual({
      x: 0,
      y: 0,
      w: 1920,
      h: 1080
    })
  })

  it('fits 9:16 inside 16:9 at zoom 0', () => {
    const r = cropRect({ ratio: '9:16', cx: 0.5, cy: 0.5, zoom: 0 }, 1920, 1080)
    expect(r.h).toBeCloseTo(1080)
    expect(r.w).toBeCloseTo(1080 * (9 / 16))
    expect(r.x).toBeCloseTo((1920 - r.w) / 2)
    expect(r.y).toBeCloseTo(0)
  })

  it('keeps the window inside the frame when panned to a corner', () => {
    const r = cropRect({ ratio: '1:1', cx: 0, cy: 0, zoom: 0 }, 1920, 1080)
    expect(r.x).toBe(0)
    expect(r.y).toBe(0)
    expect(r.x + r.w).toBeLessThanOrEqual(1920)
    expect(r.y + r.h).toBeLessThanOrEqual(1080)
  })

  it('shrinks the window when zooming in', () => {
    const fit = cropRect({ ratio: '4:3', cx: 0.5, cy: 0.5, zoom: 0 }, 1920, 1080)
    const in_ = cropRect({ ratio: '4:3', cx: 0.5, cy: 0.5, zoom: 1 }, 1920, 1080)
    expect(in_.w).toBeLessThan(fit.w)
    expect(in_.h).toBeLessThan(fit.h)
  })
})

describe('snapCropCenter', () => {
  it('rewrites cx/cy so they match the clamped window', () => {
    const snapped = snapCropCenter({ ratio: '9:16', cx: 0, cy: 0, zoom: 0 }, 1, 1)
    const r = cropRect(snapped, 1, 1)
    expect(snapped.cx).toBeCloseTo(r.x + r.w / 2)
    expect(snapped.cy).toBeCloseTo(r.y + r.h / 2)
  })
})

describe('ffmpegCropFilter', () => {
  it('returns null for original', () => {
    expect(ffmpegCropFilter(DEFAULT_CROP, 1920, 1080)).toBeNull()
  })

  it('emits even crop+scale for 9:16', () => {
    const filter = ffmpegCropFilter(
      { ratio: '9:16', cx: 0.5, cy: 0.5, zoom: 0 },
      1920,
      1080
    )
    expect(filter).toMatch(/^crop=\d+:\d+:\d+:\d+,scale=1080:1920$/)
    const nums = filter!.match(/\d+/g)!.map(Number)
    const [w, h, x, y] = nums
    expect(w % 2).toBe(0)
    expect(h % 2).toBe(0)
    expect(x + w).toBeLessThanOrEqual(1920)
    expect(y + h).toBeLessThanOrEqual(1080)
  })
})

describe('cropFromPrevious', () => {
  it('copies the last matching ratio', () => {
    const clips = [
      { title: 'Hook', crop: { ratio: '9:16' as const, cx: 0.4, cy: 0.5, zoom: 0.2 } },
      { title: 'B-roll', crop: { ratio: '4:3' as const, cx: 0.5, cy: 0.5, zoom: 0 } },
      { title: 'CTA', crop: { ...DEFAULT_CROP } }
    ]
    const { crop, fromTitle } = cropFromPrevious(clips, 2, '9:16')
    expect(fromTitle).toBe('Hook')
    expect(crop).toEqual(clips[0].crop)
  })

  it('starts centered when no previous clip used that ratio', () => {
    const { crop, fromTitle } = cropFromPrevious(
      [{ title: 'A', crop: { ...DEFAULT_CROP } }],
      0,
      '4:3'
    )
    expect(fromTitle).toBeNull()
    expect(crop).toEqual({ ratio: '4:3', cx: 0.5, cy: 0.5, zoom: 0 })
  })
})

describe('containRect', () => {
  it('letterboxes a 16:9 video in a square', () => {
    const r = containRect(100, 100, 1920, 1080)
    expect(r.w).toBeCloseTo(100)
    expect(r.h).toBeCloseTo(100 * (1080 / 1920))
    expect(r.x).toBeCloseTo(0)
    expect(r.y).toBeCloseTo((100 - r.h) / 2)
  })
})
