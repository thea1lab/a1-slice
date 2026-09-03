import { describe, it, expect } from 'vitest'
import { formatSrtTime, generateSrt, parseFfmpegDuration, parseFfmpegVideoSize, buildCutClipArgs, buildPreviewClipArgs } from '../ffmpeg'

describe('formatSrtTime', () => {
  it('formats zero', () => {
    expect(formatSrtTime(0)).toBe('00:00:00,000')
  })

  it('formats milliseconds only', () => {
    expect(formatSrtTime(500)).toBe('00:00:00,500')
  })

  it('formats seconds and millis', () => {
    expect(formatSrtTime(5123)).toBe('00:00:05,123')
  })

  it('formats minutes', () => {
    expect(formatSrtTime(90000)).toBe('00:01:30,000')
  })

  it('formats hours', () => {
    expect(formatSrtTime(3661001)).toBe('01:01:01,001')
  })
})

describe('generateSrt', () => {
  it('returns empty string for no segments', () => {
    expect(generateSrt([])).toBe('')
  })

  it('generates valid SRT for one segment', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 2000, text: 'Hello world' }
    ])
    expect(result).toBe('1\n00:00:00,000 --> 00:00:02,000\nHello world\n')
  })

  it('generates valid SRT for multiple segments', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 2000, text: 'First' },
      { startMs: 2000, endMs: 5000, text: 'Second' }
    ])
    expect(result).toBe(
      '1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n'
    )
  })

  it('trims whitespace from text', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 1000, text: '  spaced  ' }
    ])
    expect(result).toContain('spaced')
    expect(result).not.toContain('  spaced  ')
  })
})

describe('parseFfmpegDuration', () => {
  it('parses centiseconds as a fraction of a second', () => {
    expect(parseFfmpegDuration('Duration: 00:01:23.45, start: 0.000000')).toBe(
      83450
    )
  })

  it('parses three-digit fractional seconds as milliseconds', () => {
    expect(parseFfmpegDuration('  Duration: 01:00:00.500\n')).toBe(3600500)
  })

  it('returns null when duration is missing', () => {
    expect(parseFfmpegDuration('no duration here')).toBeNull()
  })
})

describe('parseFfmpegVideoSize', () => {
  it('reads width and height from a video stream line', () => {
    const stderr =
      'Stream #0:0(und): Video: h264 (High) (avc1 / 0x31637661), yuv420p, 1920x1080 [SAR 1:1 DAR 16:9]'
    expect(parseFfmpegVideoSize(stderr)).toEqual({ width: 1920, height: 1080 })
  })

  it('returns null when no video size is present', () => {
    expect(parseFfmpegVideoSize('Duration: 00:01:00.00')).toBeNull()
  })
})

describe('buildCutClipArgs', () => {
  it('re-encodes instead of stream-copying so cuts are frame-accurate', () => {
    const args = buildCutClipArgs('/in.mp4', '/out.mp4', 12.5, 30)
    expect(args).not.toContain('copy')
    expect(args).toContain('libx264')
    expect(args).toContain('aac')
    expect(args[args.indexOf('-ss') + 1]).toBe('12.5')
    expect(args[args.indexOf('-t') + 1]).toBe('30')
  })

  it('adds a crop/scale filter when provided', () => {
    const args = buildCutClipArgs(
      '/in.mp4',
      '/out.mp4',
      0,
      5,
      'crop=608:1080:656:0,scale=1080:1920'
    )
    expect(args[args.indexOf('-vf') + 1]).toBe('crop=608:1080:656:0,scale=1080:1920')
  })
})

describe('buildPreviewClipArgs', () => {
  it('cuts a small H.264 preview from the original timeline', () => {
    const args = buildPreviewClipArgs('/in.mp4', '/preview.mp4', 5, 8)
    expect(args).toContain('libx264')
    expect(args).toContain('ultrafast')
    expect(args).toContain('scale=360:-2')
    expect(args[args.indexOf('-ss') + 1]).toBe('5')
    expect(args[args.indexOf('-t') + 1]).toBe('8')
  })
})
