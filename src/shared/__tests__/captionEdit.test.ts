import { describe, expect, it } from 'vitest'
import {
  acceptAgentEdit,
  agentOutputText,
  applyEditedCaptionText,
  breakCaptionLines,
  captionEditPrompt,
  captionFileText,
  captionLogLine,
  coerceSegments,
  isNoneAnswer,
  compactDiff,
  diffCaptionLines,
  diffSummary,
  emptyAgentLogState,
  extractPrintedCaption,
  interpretAgentLine,
  visibleLogLine
} from '../captionEdit'
import type { TranscriptSegment } from '../types'

const lines: TranscriptSegment[] = [
  { startMs: 0, endMs: 2000, text: 'Hello teh world' },
  { startMs: 2000, endMs: 6000, text: 'This line is much too long for one caption' }
]

describe('caption edit prompt', () => {
  it('asks only for typos, shorter lines, and the user’s note', () => {
    const prompt = captionEditPrompt(
      {
        fixTypos: true,
        breakLines: true,
        wordsPerLine: 8,
        note: 'The name is Anna'
      },
      captionFileText(lines)
    )
    expect(prompt).toContain('Edit this transcript.')
    expect(prompt).toContain('Fix typos')
    expect(prompt).toContain('longer than 8 words')
    expect(prompt).toContain('The name is Anna')
    expect(prompt).toContain('Print only the edited transcript.')
    expect(prompt).toContain('0:00  Hello teh world')
    expect(prompt).not.toContain('captions.txt')
    expect(prompt).not.toContain('CAPTIONS_JSON')
  })

  it('asks for NONE when it should not break lines', () => {
    const prompt = captionEditPrompt(
      { fixTypos: true, breakLines: false, wordsPerLine: 8, note: '' },
      '0:00  Hello teh world'
    )
    expect(prompt).toContain('print exactly NONE')
    expect(prompt).toContain('Do not join or split lines.')
    expect(isNoneAnswer('NONE')).toBe(true)
    expect(isNoneAnswer("I'll split the line.")).toBe(false)
  })
})

describe('breakCaptionLines', () => {
  it('splits a long line into even pieces and keeps every word', () => {
    const [segment] = lines.slice(1)
    expect(breakCaptionLines([segment], 8)).toEqual([
      { startMs: 2000, endMs: 4222, text: 'This line is much too' },
      { startMs: 4222, endMs: 6000, text: 'long for one caption' }
    ])
  })

  it('leaves a line that is already short enough', () => {
    const segment = { startMs: 0, endMs: 1000, text: 'one two three four five six seven eight' }
    expect(breakCaptionLines([segment], 8)).toEqual([segment])
  })

  it('keeps a line that starts while the previous line is still going', () => {
    const before: TranscriptSegment[] = [
      {
        startMs: 171000,
        endMs: 176500,
        text: 'Mas só você seleciona clique aqui e selecione as imagens que vocês querem'
      },
      { startMs: 173960, endMs: 175840, text: 'e selecionem as imagens' }
    ]
    const split = breakCaptionLines(before, 8)
    expect(split.every((segment) => segment.endMs > segment.startMs)).toBe(true)
    const parsed = applyEditedCaptionText(before, captionFileText(split))
    expect(parsed).toMatchObject({ segments: expect.any(Array) })
    if (!('segments' in parsed)) return
    expect(parsed.segments.every((segment) => segment.endMs > segment.startMs)).toBe(true)
    expect(coerceSegments(parsed.segments)?.map((segment) => segment.text).join(' ')).toContain(
      'e selecionem as imagens'
    )
  })

  it('breaks a very long line into pieces of at most eight words', () => {
    const words = Array.from({ length: 31 }, (_, index) => `w${index + 1}`)
    const result = breakCaptionLines([{ startMs: 0, endMs: 31000, text: words.join(' ') }], 8)
    expect(result.map((segment) => segment.text.split(' ').length)).toEqual([8, 8, 8, 7])
    expect(result.map((segment) => segment.text).join(' ')).toBe(words.join(' '))
    expect(result[0].startMs).toBe(0)
    expect(result[result.length - 1].endMs).toBe(31000)
  })
})

describe('applyEditedCaptionText', () => {
  it('keeps the times when the file only fixes a typo', () => {
    const edited = captionFileText([
      { startMs: 0, endMs: 2000, text: 'Hello the world' },
      { startMs: 2000, endMs: 6000, text: 'This line is much too long for one caption' }
    ])
    expect(applyEditedCaptionText(lines, edited)).toEqual({
      segments: [
        { startMs: 0, endMs: 2000, text: 'Hello the world' },
        { startMs: 2000, endMs: 6000, text: 'This line is much too long for one caption' }
      ]
    })
  })

  it('splits the time when a long line becomes two lines', () => {
    const edited = ['0:00  Hello the world', '0:02  This line is much', 'too long for one caption'].join('\n')
    expect(applyEditedCaptionText(lines, edited)).toEqual({
      segments: [
        { startMs: 0, endMs: 2000, text: 'Hello the world' },
        { startMs: 2000, endMs: 3778, text: 'This line is much' },
        { startMs: 3778, endMs: 6000, text: 'too long for one caption' }
      ]
    })
  })

  it('refuses a rewrite that drops most of the words', () => {
    expect(applyEditedCaptionText(lines, '0:00  Hello\n')).toEqual({
      error: 'The agent changed too much of the file. The lines were left as they are.'
    })
  })

  it('keeps the original times when the file is unchanged', () => {
    expect(applyEditedCaptionText(lines, captionFileText(lines))).toEqual({ segments: lines })
  })
})

describe('extractPrintedCaption', () => {
  it('keeps a split line that has no time of its own', () => {
    const printed = extractPrintedCaption(
      [
        'I will fix the typo.',
        '0:00  Hello the world',
        '0:02  This line is much too long',
        'for one caption',
        '',
        'Done.'
      ].join('\n')
    )
    expect(printed).toBe(['0:00  Hello the world', '0:02  This line is much too long', 'for one caption'].join('\n'))
    expect(applyEditedCaptionText(lines, printed ?? '')).toEqual({
      segments: [
        { startMs: 0, endMs: 2000, text: 'Hello the world' },
        { startMs: 2000, endMs: 4667, text: 'This line is much too long' },
        { startMs: 4667, endMs: 6000, text: 'for one caption' }
      ]
    })
  })

  it('uses the last transcript when an earlier copy is still in the answer', () => {
    const printed = extractPrintedCaption(
      [
        '0:00  Hello teh world',
        '0:02  This line is much too long for one caption',
        '',
        '0:00  Hello the world',
        '0:02  This line is much too long',
        'for one caption'
      ].join('\n')
    )
    expect(printed).toBe(['0:00  Hello the world', '0:02  This line is much too long', 'for one caption'].join('\n'))
  })
})

describe('caption log', () => {
  it('shows the note and hides the printed transcript', () => {
    const state = { hidingFile: false }
    expect(captionLogLine('I will fix the typo.', state)).toBe('I will fix the typo.')
    expect(captionLogLine('0:00  Hello the world', state)).toBe('Writing the edited transcript.')
    expect(captionLogLine('for one caption', state)).toBeNull()
  })
})

describe('acceptAgentEdit', () => {
  it('reads the JSON block and keeps a small correction', () => {
    const output = [
      'NOTE: At 00:00, fixed "teh" to "the".',
      'CAPTIONS_JSON_START',
      JSON.stringify([
        { startMs: 0, endMs: 2000, text: 'Hello the world' },
        { startMs: 2000, endMs: 4000, text: 'This line is much' },
        { startMs: 4000, endMs: 6000, text: 'too long for one caption' }
      ]),
      'CAPTIONS_JSON_END'
    ].join('\n')
    const result = acceptAgentEdit(lines, output)
    expect(result).toEqual({
      segments: [
        { startMs: 0, endMs: 2000, text: 'Hello the world' },
        { startMs: 2000, endMs: 4000, text: 'This line is much' },
        { startMs: 4000, endMs: 6000, text: 'too long for one caption' }
      ]
    })
  })

  it('refuses a rewrite that drops most of the words', () => {
    const output = `CAPTIONS_JSON_START\n${JSON.stringify([{ startMs: 0, endMs: 2000, text: 'Hello' }])}\nCAPTIONS_JSON_END`
    expect(acceptAgentEdit(lines, output)).toEqual({
      error: 'The agent removed too many words. The lines were left as they are.'
    })
  })

  it('hides the JSON from the log and keeps the notes', () => {
    const state = { hideJson: false }
    expect(visibleLogLine('NOTE: At 00:00, fixed "teh" to "the".', state)).toBe(
      'At 00:00, fixed "teh" to "the".'
    )
    expect(visibleLogLine('CAPTIONS_JSON_START', state)).toBe('Writing the edited captions.')
    expect(visibleLogLine('[{"text":"hidden"}]', state)).toBeNull()
    expect(visibleLogLine('CAPTIONS_JSON_END', state)).toBeNull()
  })
})

describe('caption diff', () => {
  it('shows a removed line and the line that replaced it', () => {
    const rows = diffCaptionLines(['00:00  Hello teh'], ['00:00  Hello the'])
    expect(rows.map((row) => row.kind)).toEqual(['remove', 'add'])
    expect(diffSummary(rows)).toBe('Removed 1 line. Added 1 line.')
    const compact = compactDiff(
      diffCaptionLines(
        ['00:00  Same', '00:02  Old', '00:04  After'],
        ['00:00  Same', '00:02  New', '00:04  After']
      ),
      0
    )
    expect(compact.map((row) => row.kind)).toEqual(['gap', 'remove', 'add'])
  })
})

describe('agent log stream', () => {
  it('turns a Claude tool call into a plain sentence and keeps the result text', () => {
    const state = emptyAgentLogState()
    const reading = interpretAgentLine(
      JSON.stringify({
        type: 'assistant',
        message: { content: [{ type: 'tool_use', name: 'Read' }] }
      }),
      state,
      true
    )
    expect(reading).toEqual(['Reading the captions.'])
    const finished = interpretAgentLine(
      JSON.stringify({
        type: 'result',
        result: 'NOTE: At 00:00, fixed a typo.\nCAPTIONS_JSON_START\n[]\nCAPTIONS_JSON_END'
      }),
      state,
      true
    )
    expect(finished).toEqual(['At 00:00, fixed a typo.', 'Writing the edited captions.'])
    expect(agentOutputText(state)).toContain('CAPTIONS_JSON_START')
  })

  it('does not repeat notes that were already shown', () => {
    const state = emptyAgentLogState()
    interpretAgentLine(
      JSON.stringify({
        type: 'assistant',
        message: { content: [{ type: 'text', text: 'NOTE: At 00:00, fixed a typo.' }] }
      }),
      state,
      true
    )
    const again = interpretAgentLine(
      JSON.stringify({
        type: 'result',
        result: 'NOTE: At 00:00, fixed a typo.\nCAPTIONS_JSON_START\n[]\nCAPTIONS_JSON_END'
      }),
      state,
      true
    )
    expect(again).toEqual([])
    expect(agentOutputText(state)).toContain('CAPTIONS_JSON_START')
  })
})
