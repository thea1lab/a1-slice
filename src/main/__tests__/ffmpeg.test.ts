import { describe, it, expect } from 'vitest'
import { formatSrtTime, generateSrt, parseFfmpegDuration, buildCutClipArgs } from '../ffmpeg'

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

describe('buildCutClipArgs', () => {
  it('re-encodes instead of stream-copying so cuts are frame-accurate', () => {
    const args = buildCutClipArgs('/in.mp4', '/out.mp4', 12.5, 30)
    expect(args).not.toContain('copy')
    expect(args).toContain('libx264')
    expect(args).toContain('aac')
    expect(args[args.indexOf('-ss') + 1]).toBe('12.5')
    expect(args[args.indexOf('-t') + 1]).toBe('30')
  })
})
