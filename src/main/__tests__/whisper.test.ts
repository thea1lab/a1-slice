import { describe, it, expect } from 'vitest'
import { parseTimestamp, parseWhisperJson } from '../whisper'

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
