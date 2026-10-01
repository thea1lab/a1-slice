import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import ReframeScreen from '../screens/ReframeScreen'

function screen(): string {
  return renderToStaticMarkup(
    createElement(ReframeScreen, {
      videoPath: '/videos/talk.mp4',
      durationMs: 125_000,
      framing: {},
      exportStage: 'idle',
      exportMessage: '',
      exportPercent: 0,
      outputDir: null,
      exportError: null,
      onFramingChange: () => {},
      onExport: () => {},
      onOpenFolder: () => {}
    })
  )
}

describe('ReframeScreen', () => {
  it('has a play control and a bar that shows the current time and the end', () => {
    const html = screen()
    expect(html).toContain('Press space to play and pause')
    expect(html).toContain('aria-label="Play"')
    expect(html).toContain('aria-label="Video time"')
    expect(html).toContain('aria-label="Current time"')
    expect(html).toContain('0:00')
    expect(html).toContain('2:05')
  })
})
