import { describe, expect, it } from 'vitest'
import { formatPlaybackClock, planScrub, playbackMsAt } from '../lib/playback'

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

describe('planScrub', () => {
  it('seeks immediately when the player is idle', () => {
    expect(planScrub(0, 12.5, false)).toEqual({ seekTo: 12.5, queued: null })
  })

  it('keeps the latest time while a seek is still painting', () => {
    expect(planScrub(1, 4, true)).toEqual({ seekTo: null, queued: 4 })
    expect(planScrub(1, 9, true)).toEqual({ seekTo: null, queued: 9 })
  })

  it('does not seek again when the frame is already there', () => {
    expect(planScrub(5, 5.02, false)).toEqual({ seekTo: null, queued: null })
  })

  it('seeks to the queued time once the player is free', () => {
    const queued = planScrub(1, 9, true).queued
    expect(queued).toBe(9)
    expect(planScrub(1, queued ?? 0, false)).toEqual({ seekTo: 9, queued: null })
  })
})
