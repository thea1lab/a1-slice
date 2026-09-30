import { describe, expect, it } from 'vitest'
import { formatPlaybackClock, playbackMsAt } from '../lib/playback'

describe('playback clock', () => {
  it('shows minutes and seconds, and hours when the video is longer', () => {
    expect(formatPlaybackClock(0)).toBe('0:00')
    expect(formatPlaybackClock(65000)).toBe('1:05')
    expect(formatPlaybackClock(3723000)).toBe('1:02:03')
  })

  it('maps a click on the bar to a time between the start and the end', () => {
    expect(playbackMsAt(0, 0, 200, 0, 10000)).toBe(0)
    expect(playbackMsAt(100, 0, 200, 0, 10000)).toBe(5000)
    expect(playbackMsAt(200, 0, 200, 0, 10000)).toBe(10000)
    expect(playbackMsAt(-20, 0, 200, 1000, 3000)).toBe(1000)
    expect(playbackMsAt(999, 0, 200, 1000, 3000)).toBe(3000)
  })
})
