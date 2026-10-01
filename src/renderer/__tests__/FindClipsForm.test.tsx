import { createElement } from 'react'
import { renderToStaticMarkup } from 'react-dom/server'
import { describe, expect, it } from 'vitest'
import FindClipsForm from '../components/FindClipsForm'

const segments = [{ startMs: 0, endMs: 1000, text: 'Hello there' }]

function form(extra: Partial<Parameters<typeof FindClipsForm>[0]> = {}): string {
  return renderToStaticMarkup(
    createElement(FindClipsForm, {
      segments,
      videoPath: '/videos/talk.mp4',
      agents: [
        { id: 'grok', label: 'Grok 4.7' },
        { id: 'agy', label: 'Gemini 3.8 Flash' }
      ],
      agentId: 'grok',
      userHint: '',
      analyzing: false,
      analyzePercent: 0,
      analyzeMessage: '',
      error: null,
      onAgentChange: () => {},
      onUserHintChange: () => {},
      onAnalyze: () => {},
      onCancel: () => {},
      onAddRange: () => {},
      ...extra
    })
  )
}

describe('FindClipsForm', () => {
  it('asks an installed agent instead of an API key', () => {
    const html = form()
    expect(html).toContain('Which agent')
    expect(html).toContain('Grok 4.7')
    expect(html).toContain('Gemini 3.8 Flash')
    expect(html).toContain('Find clips')
    expect(html).toContain('Mark a part yourself')
    expect(html).toContain('What should it look for?')
    expect(html).not.toContain('API key')
    expect(html).not.toContain('OpenAI')
    expect(html).not.toContain('OpenCode')
    expect(html).not.toContain('>Claude<')
  })

  it('waits until an agent is chosen', () => {
    const html = form({ agents: [], agentId: null })
    expect(html).toContain('This computer has no agent to run.')
    expect(html).toContain('disabled')
  })

  it('shows the agent status while it reads', () => {
    const html = form({
      analyzing: true,
      analyzePercent: 20,
      analyzeMessage: 'Starting Grok 4.7.'
    })
    expect(html).toContain('Starting Grok 4.7.')
    expect(html).toContain('Cancel')
    expect(html).not.toContain('Find clips')
  })
})
