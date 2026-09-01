import { describe, it, expect } from 'vitest'
import { parseTimestamp, parseWhisperJson, mergeChunkSegments } from '../whisper'

describe('parseTimestamp', () => {
  it('parses zero timestamp', () => {
    expect(parseTimestamp('00:00:00.000')).toBe(0)
  })

  it('parses seconds and millis', () => {
    expect(parseTimestamp('00:00:05.123')).toBe(5123)
  })

  it('parses minutes', () => {
    expect(parseTimestamp('00:01:30.000')).toBe(90000)
  })

  it('parses hours', () => {
    expect(parseTimestamp('01:01:01.001')).toBe(3661001)
  })

  it('handles comma separator', () => {
    expect(parseTimestamp('00:00:05,123')).toBe(5123)
  })

  it('handles missing millis', () => {
    expect(parseTimestamp('00:01:00.0')).toBe(60000)
  })
})

describe('parseWhisperJson', () => {
  it('parses whisper JSON output', () => {
    const json = JSON.stringify({
      transcription: [
        {
          timestamps: { from: '00:00:00.000', to: '00:00:05.000' },
          text: ' Hello world'
        },
        {
          timestamps: { from: '00:00:05.000', to: '00:00:10.500' },
          text: ' How are you'
        }
      ]
    })
    const result = parseWhisperJson(json)
    expect(result).toHaveLength(2)
    expect(result[0]).toEqual({
      startMs: 0,
      endMs: 5000,
      text: ' Hello world'
    })
    expect(result[1]).toEqual({
      startMs: 5000,
      endMs: 10500,
      text: ' How are you'
    })
  })

  it('handles empty transcription', () => {
    const json = JSON.stringify({ transcription: [] })
    expect(parseWhisperJson(json)).toEqual([])
  })
})

describe('mergeChunkSegments', () => {
  it('keeps the earlier chunk in the first half of the overlap', () => {
    const overlapMs = 15000
    const merged = mergeChunkSegments(
      [
        {
          offsetMs: 0,
          durationMs: 180000,
          segments: [
            { startMs: 0, endMs: 2000, text: 'start' },
            { startMs: 170000, endMs: 172000, text: 'from-first' }
          ]
        },
        {
          offsetMs: 165000,
          durationMs: 180000,
          segments: [
            { startMs: 2000, endMs: 4000, text: 'from-second-early' },
            { startMs: 20000, endMs: 22000, text: 'from-second' }
          ]
        }
      ],
      overlapMs
    )
    const texts = merged.map((s) => s.text)
    expect(texts).toContain('start')
    expect(texts).toContain('from-first')
    expect(texts).toContain('from-second')
    expect(texts).not.toContain('from-second-early')
  })

  it('applies chunk offsets to local timestamps', () => {
    const merged = mergeChunkSegments(
      [
        {
          offsetMs: 0,
          durationMs: 10000,
          segments: [{ startMs: 1000, endMs: 2000, text: 'a' }]
        },
        {
          offsetMs: 8000,
          durationMs: 10000,
          segments: [{ startMs: 3000, endMs: 4000, text: 'b' }]
        }
      ],
      2000
    )
    expect(merged.find((s) => s.text === 'a')?.startMs).toBe(1000)
    expect(merged.find((s) => s.text === 'b')?.startMs).toBe(11000)
  })
})
