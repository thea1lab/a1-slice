import { describe, it, expect } from 'vitest'
import { parseByteRange, rangeResponseMeta, videoMimeForExt } from '../httpRange'

describe('parseByteRange', () => {
  const size = 1000

  it('returns null when there is no Range header so the file is served in full', () => {
    expect(parseByteRange(null, size)).toBeNull()
    expect(parseByteRange(undefined, size)).toBeNull()
    expect(parseByteRange('', size)).toBeNull()
  })

  it('parses an inclusive closed range', () => {
    expect(parseByteRange('bytes=0-499', size)).toEqual({ start: 0, end: 499 })
  })

  it('parses an open-ended range used by the media element to seek', () => {
    expect(parseByteRange('bytes=200-', size)).toEqual({ start: 200, end: 999 })
  })

  it('parses a suffix range', () => {
    expect(parseByteRange('bytes=-100', size)).toEqual({ start: 900, end: 999 })
  })

  it('parses bytes=0- as the whole file', () => {
    expect(parseByteRange('bytes=0-', size)).toEqual({ start: 0, end: 999 })
  })

  it('clamps the end to the last byte', () => {
    expect(parseByteRange('bytes=10-9999', size)).toEqual({ start: 10, end: 999 })
  })

  it('rejects an unsatisfiable start', () => {
    expect(parseByteRange('bytes=1000-1001', size)).toBeNull()
    expect(parseByteRange('bytes=5000-', size)).toBeNull()
  })
})

describe('rangeResponseMeta', () => {
  it('sends Accept-Ranges on a full-file response so the player can seek later', () => {
    expect(rangeResponseMeta(1000, null, 'video/mp4')).toEqual({
      status: 200,
      headers: {
        'Content-Type': 'video/mp4',
        'Content-Length': '1000',
        'Accept-Ranges': 'bytes'
      }
    })
  })

  it('sends 206 with Content-Range for a partial request', () => {
    expect(rangeResponseMeta(1000, { start: 200, end: 399 }, 'video/mp4')).toEqual({
      status: 206,
      headers: {
        'Content-Type': 'video/mp4',
        'Content-Length': '200',
        'Content-Range': 'bytes 200-399/1000',
        'Accept-Ranges': 'bytes'
      }
    })
  })
})

describe('videoMimeForExt', () => {
  it('maps preview mp4 files to video/mp4', () => {
    expect(videoMimeForExt('.mp4')).toBe('video/mp4')
  })
})
