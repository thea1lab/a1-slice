import { describe, expect, it } from 'vitest'
import {
  formatTranscriptText,
  parseCaptions,
  parseClipsCache,
  parseFraming,
  parseCaptionDocument,
  parseSrt,
  parseTranscriptLines,
  previewCacheKey,
  rememberRecent,
  toStoredClip,
  transcriptTextPath,
  withClipStatus
} from '../project'

describe('parseClipsCache', () => {
  it('keeps crop and a dropped clip from a current file', () => {
    const clips = parseClipsCache([
      {
        title: 'Hook',
        startMs: 0,
        endMs: 4000,
        approved: false,
        crop: { ratio: '9:16', cx: 0.4, cy: 0.5, zoom: 0.2 }
      }
    ])
    expect(clips[0].approved).toBe(false)
    expect(clips[0].crop).toEqual({ ratio: '9:16', cx: 0.4, cy: 0.5, zoom: 0.2 })
  })

  it('loads an older file that only has title and times', () => {
    const clips = parseClipsCache([{ title: 'Old', startMs: 10, endMs: 20 }])
    expect(clips).toEqual([{ title: 'Old', startMs: 10, endMs: 20 }])
    expect(withClipStatus(clips)[0].approved).toBe(true)
    expect(withClipStatus(clips)[0].crop?.ratio).toBe('original')
  })
})

describe('toStoredClip', () => {
  it('drops the ui id and records approval', () => {
    expect(
      toStoredClip({
        id: '9',
        title: 'A',
        startMs: 1,
        endMs: 2,
        approved: true
      })
    ).toEqual({ title: 'A', startMs: 1, endMs: 2, approved: true })
  })
})

describe('parseFraming', () => {
  it('returns the saved story frame', () => {
    const framing = parseFraming({
      '9:16': { ratio: '9:16', cx: 0.42, cy: 0.5, zoom: 0.1 },
      nope: { ratio: '9:16', cx: 0.1, cy: 0.1, zoom: 0 }
    })
    expect(framing['9:16']).toEqual({ ratio: '9:16', cx: 0.42, cy: 0.5, zoom: 0.1 })
    expect(framing['16:9']).toBeUndefined()
  })
})

describe('captions round trip', () => {
  const segments = [
    { startMs: 0, endMs: 2000, text: 'Hello world' },
    { startMs: 2000, endMs: 5000, text: 'Second' }
  ]

  it('parses an srt back into segments', () => {
    const srt = [
      '1',
      '00:00:00,000 --> 00:00:02,000',
      'Hello world',
      '',
      '2',
      '00:00:02,000 --> 00:00:05,000',
      'Second',
      ''
    ].join('\n')
    expect(parseSrt(srt)).toEqual(segments)
  })

  it('reads a saved caption project', () => {
    expect(
      parseCaptions({
        source: 'manual',
        look: 'burn',
        style: { color: 'yellow', position: 'top', size: 'medium', font: 'serif' },
        cues: segments
      })
    ).toEqual({
      source: 'manual',
      look: 'burn',
      style: { color: 'yellow', position: 'top', size: 'medium', font: 'serif' },
      cues: segments
    })
  })

  it('reads a chosen caption file only for pasted words', () => {
    expect(
      parseCaptions({
        source: 'manual',
        look: 'burn',
        cues: segments,
        filePath: '/videos/other.srt'
      })?.filePath
    ).toBe('/videos/other.srt')
    expect(
      parseCaptions({
        source: 'transcript',
        look: 'srt',
        cues: [],
        filePath: '/videos/other.srt'
      })?.filePath
    ).toBeUndefined()
  })

  it('reads a transcript file and prefers a subtitle file', () => {
    expect(parseTranscriptLines('0:00  Hello world\n0:02  Second\n')).toEqual([
      { startMs: 0, endMs: 2000, text: 'Hello world' },
      { startMs: 2000, endMs: 4000, text: 'Second' }
    ])
    expect(parseTranscriptLines('1:02:03  Hello').map((line) => line.startMs)).toEqual([3723000])
    const srt = ['1', '00:00:01,000 --> 00:00:03,000', 'Hello', ''].join('\n')
    expect(parseCaptionDocument(srt)).toEqual([{ startMs: 1000, endMs: 3000, text: 'Hello' }])
    expect(parseCaptionDocument('0:12  Hello there')).toEqual([
      { startMs: 12000, endMs: 14000, text: 'Hello there' }
    ])
  })

  it('keeps an older burn preset as a size', () => {
    expect(parseCaptions({ source: 'manual', look: 'burn-large', cues: segments })?.style.size).toBe('large')
    expect(parseCaptions({ source: 'manual', look: 'burn-small', cues: [] })?.style).toMatchObject({
      color: 'white',
      position: 'bottom',
      size: 'small',
      font: 'sans'
    })
    expect(parseCaptions({ source: 'transcript', look: 'srt', cues: [] })?.look).toBe('srt')
  })

  it('writes a readable transcript', () => {
    expect(formatTranscriptText(segments)).toContain('0:00  Hello world')
    expect(formatTranscriptText(segments)).toContain('0:02  Second')
  })

  it('names the readable transcript beside the video', () => {
    expect(transcriptTextPath('/videos/talk.mp4')).toBe('/videos/talk-transcript.txt')
    expect(transcriptTextPath('C:\\videos\\talk.mov')).toBe('C:\\videos\\talk-transcript.txt')
    expect(transcriptTextPath('/videos/my.clip.mkv')).toBe('/videos/my.clip-transcript.txt')
    expect(transcriptTextPath('talk.webm')).toBe('talk-transcript.txt')
  })
})

describe('rememberRecent', () => {
  it('puts the newest path first and caps the list', () => {
    const paths = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h']
    expect(rememberRecent(paths, 'c', 8)).toEqual(['c', 'a', 'b', 'd', 'e', 'f', 'g', 'h'])
    expect(rememberRecent(paths, 'new', 3)).toEqual(['new', 'a', 'b'])
  })
})

describe('previewCacheKey', () => {
  it('stays stable for the same file and range', () => {
    const key = previewCacheKey('/v.mp4', 10, 20, 0, 5000)
    expect(previewCacheKey('/v.mp4', 10, 20, 0, 5000)).toBe(key)
  })

  it('changes when the range or the file changes', () => {
    const key = previewCacheKey('/v.mp4', 10, 20, 0, 5000)
    expect(previewCacheKey('/v.mp4', 10, 20, 1000, 5000)).not.toBe(key)
    expect(previewCacheKey('/v.mp4', 10, 21, 0, 5000)).not.toBe(key)
  })
})
