import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import ReviewScreen from '../screens/ReviewScreen'
import type { ClipSegmentWithStatus } from '../../shared/types'

const clips: ClipSegmentWithStatus[] = [
  {
    id: '0',
    title: 'The elephant',
    startMs: 10_000,
    endMs: 40_000,
    approved: true
  }
]

function review(): string {
  return renderToStaticMarkup(
    createElement(ReviewScreen, {
      clips,
      videoPath: '/videos/talk.mp4',
      videoDurationMs: 600_000,
      onToggle: () => {},
      onUpdateClipTimes: () => {},
      onSlice: () => {},
      onFindAgain: () => {},
      onAddRange: () => {}
    })
  )
}

describe('ReviewScreen', () => {
  it('keeps a start handle and an end handle on the video bar', () => {
    const html = review()
    expect(html).toContain('The elephant')
    expect(html).toContain('aria-label="Start"')
    expect(html).toContain('aria-label="End"')
    expect(html).toContain('aria-label="Current time"')
    expect(html).toContain('aria-label="Start time 0:10"')
    expect(html).toContain('aria-label="End time 0:40"')
    expect(html).toContain('data-clip-time="start"')
    expect(html).toContain('data-clip-time="end"')
    expect(html).not.toContain('10:00')
    expect(html).toContain('one minute before')
    expect(html).toContain('Export 1 clip')
  })

  it('does not ask about framing, volume, fullscreen, or subtitles', () => {
    const html = review()
    expect(html).not.toContain('Subtitles on export')
    expect(html).not.toContain('Volume')
    expect(html).not.toContain('Fullscreen')
    expect(html).not.toContain('−1s')
    expect(html).not.toContain('Story')
    expect(html).not.toContain('Original')
    expect(html).toContain('includes the words')
  })
})
