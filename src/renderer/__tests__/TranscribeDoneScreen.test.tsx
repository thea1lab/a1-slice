import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import TranscribeDoneScreen from '../screens/TranscribeDoneScreen'

describe('TranscribeDoneScreen', () => {
  it('shows where the transcript file is and sends the next step home', () => {
    const html = renderToStaticMarkup(
      createElement(TranscribeDoneScreen, {
        videoPath: '/videos/talk.mp4',
        segments: [
          { startMs: 0, endMs: 1000, text: 'hello there' },
          { startMs: 10000, endMs: 12000, text: 'second line' }
        ],
        onOpen: async () => ({ success: true }),
        onHome: () => {}
      })
    )

    expect(html).toContain('Transcript saved')
    expect(html).toContain('2 lines are saved in a text file in the same folder as the video')
    expect(html).toContain('/videos/talk-transcript.txt')
    expect(html).toContain('hello there')
    expect(html).toContain('Open transcription')
    expect(html).toContain('Back to home')
    expect(html).toContain('find the best parts')
    expect(html).toContain('add captions')
    expect(html).not.toContain('Find best parts')
    expect(html).not.toContain('Add captions')
    expect(html).not.toContain('Save a copy')
  })
})
