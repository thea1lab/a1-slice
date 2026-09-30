import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import CaptionsScreen from '../screens/CaptionsScreen'
import type { CaptionStyle } from '../../shared/types'

const style: CaptionStyle = {
  color: 'white',
  position: 'bottom',
  size: 'medium',
  font: 'sans'
}

function screen(outputDir: string | null, stage: 'idle' | 'done' | 'cutting' = 'done'): string {
  return renderToStaticMarkup(
    createElement(CaptionsScreen, {
      videoPath: '/videos/talk.mp4',
      durationMs: 1000,
      segments: [{ startMs: 0, endMs: 1000, text: 'Hello there' }],
      captions: { source: 'transcript', look: 'burn', style, cues: [] },
      exportStage: stage,
      exportMessage: '',
      exportPercent: stage === 'done' ? 100 : 0,
      outputDir,
      exportError: null,
      onChange: () => {},
      onExport: () => {},
      onFixWords: () => {}
    })
  )
}

describe('CaptionsScreen', () => {
  it('keeps Export captions and shows the finished path beside it', () => {
    const html = screen('/videos/a1slice-talk-captions-1')
    expect(html).toContain('Export captions')
    expect(html).toContain('/videos/a1slice-talk-captions-1')
    expect(html).toContain('aria-label="Video time"')
    expect(html).toContain('0:00')
    expect(html).toContain('0:01')
    expect(html).not.toContain('Show file')
    expect(html).not.toContain('The video is in this folder')
  })

  it('hides the path until the render is finished', () => {
    const idle = screen(null, 'idle')
    const cutting = screen('/videos/a1slice-talk-captions-1', 'cutting')
    expect(idle).toContain('Export captions')
    expect(idle).not.toContain('/videos/a1slice-talk-captions-1')
    expect(cutting).toContain('Export captions')
    expect(cutting).not.toContain('/videos/a1slice-talk-captions-1')
    expect(cutting).not.toContain('Show file')
  })
})
