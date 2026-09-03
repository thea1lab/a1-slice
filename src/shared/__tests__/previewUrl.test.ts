import { describe, it, expect } from 'vitest'
import {
  toPreviewSrc,
  pathFromPreviewUrl,
  paddedPreviewRange,
  previewCoversRange
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
