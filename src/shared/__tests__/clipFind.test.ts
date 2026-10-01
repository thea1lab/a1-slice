import { describe, expect, it } from 'vitest'
import { clipFindPrompt, formatClipTranscript } from '../clipFind'

const segments = [
  { startMs: 0, endMs: 5000, text: '  Hello  ' },
  { startMs: 5000, endMs: 10000, text: 'World' }
]

describe('clip find prompt', () => {
  it('numbers each transcript line the way the clip parser expects', () => {
    expect(formatClipTranscript(segments)).toBe(
      '[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello\n[#1 00:00:05 -> 00:00:10 | 5000 -> 10000] World'
    )
  })

  it('asks for start and end line ids and includes the hint when one is written', () => {
    const prompt = clipFindPrompt(segments, 'the part where the elephant confuses the model')
    expect(prompt).toContain('[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello')
    expect(prompt).toContain('start_id')
    expect(prompt).toContain('end_id')
    expect(prompt).toContain('20-90 seconds')
    expect(prompt).toContain(
      'Hard constraint from the user — discard anything that does not match: the part where the elephant confuses the model'
    )
  })

  it('leaves the hint out when the box is empty', () => {
    expect(clipFindPrompt(segments, '   ')).not.toContain('Hard constraint')
  })
})
