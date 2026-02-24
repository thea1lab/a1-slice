import { describe, it, expect } from 'vitest'
import {
  msToTimecode,
  formatTranscriptForLLM,
  parseLLMResponse
} from '../analyzer'

describe('msToTimecode', () => {
  it('formats zero', () => {
    expect(msToTimecode(0)).toBe('00:00:00')
  })

  it('formats seconds', () => {
    expect(msToTimecode(5000)).toBe('00:00:05')
  })

  it('formats minutes and seconds', () => {
    expect(msToTimecode(90000)).toBe('00:01:30')
  })

  it('formats hours', () => {
    expect(msToTimecode(3661000)).toBe('01:01:01')
  })

  it('truncates milliseconds', () => {
    expect(msToTimecode(5999)).toBe('00:00:05')
  })
})

describe('formatTranscriptForLLM', () => {
  it('formats segments with timecodes and milliseconds', () => {
    const result = formatTranscriptForLLM([
      { startMs: 0, endMs: 5000, text: 'Hello' },
      { startMs: 5000, endMs: 10000, text: 'World' }
    ])
    expect(result).toBe(
      '[00:00:00 -> 00:00:05 | 0 -> 5000] Hello\n[00:00:05 -> 00:00:10 | 5000 -> 10000] World'
    )
  })

  it('returns empty string for no segments', () => {
    expect(formatTranscriptForLLM([])).toBe('')
  })

  it('trims whitespace from text', () => {
    const result = formatTranscriptForLLM([
      { startMs: 0, endMs: 1000, text: '  spaced  ' }
    ])
    expect(result).toBe('[00:00:00 -> 00:00:01 | 0 -> 1000] spaced')
  })
})

describe('parseLLMResponse', () => {
  it('parses a clean JSON array', () => {
    const text = JSON.stringify([
      { title: 'Clip 1', start_ms: 0, end_ms: 30000 },
      { title: 'Clip 2', start_ms: 60000, end_ms: 120000 }
    ])
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Clip 1', startMs: 0, endMs: 30000 },
      { title: 'Clip 2', startMs: 60000, endMs: 120000 }
    ])
  })

  it('extracts JSON from markdown code block', () => {
    const text = `Here are the clips:
\`\`\`json
[{"title": "Great moment", "start_ms": 1000, "end_ms": 5000}]
\`\`\``
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Great moment', startMs: 1000, endMs: 5000 }
    ])
  })

  it('extracts JSON with surrounding text', () => {
    const text = 'Based on the transcript, here are the best clips:\n[{"title": "Test", "start_ms": 0, "end_ms": 1000}]\nHope that helps!'
    const result = parseLLMResponse(text)
    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Test')
  })

  it('throws on missing JSON', () => {
    expect(() => parseLLMResponse('No clips found')).toThrow(
      'No JSON array found'
    )
  })

  it('parses category field when present', () => {
    const text = JSON.stringify([
      { title: 'Main topic clip', start_ms: 0, end_ms: 60000, category: 'related' },
      { title: 'Self-contained insight', start_ms: 120000, end_ms: 180000, category: 'standalone' }
    ])
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Main topic clip', startMs: 0, endMs: 60000, category: 'related' },
      { title: 'Self-contained insight', startMs: 120000, endMs: 180000, category: 'standalone' }
    ])
  })

  it('omits category when not present in response', () => {
    const text = JSON.stringify([
      { title: 'No category', start_ms: 0, end_ms: 30000 }
    ])
    const result = parseLLMResponse(text)
    expect(result[0].category).toBeUndefined()
  })

  it('ignores invalid category values', () => {
    const text = JSON.stringify([
      { title: 'Bad category', start_ms: 0, end_ms: 30000, category: 'invalid' }
    ])
    const result = parseLLMResponse(text)
    expect(result[0].category).toBeUndefined()
  })
})
