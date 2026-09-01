import { describe, it, expect } from 'vitest'
import {
  msToTimecode,
  formatTranscriptForLLM,
  parseLLMResponse,
  parseTopicSegments,
  validateSegmentation,
  fallbackToTimeChunks,
  getSegmentsInRange,
  generateTranscriptPreview,
  deduplicateClips,
  budgetCandidates,
  enforceCoverage,
  openCodeApiKind,
  normalizeOpenCodeModel,
  extractJsonArray,
  analyzeTranscript
} from '../analyzer'

// --- Existing tests (unchanged) ---

describe('msToTimecode', () => {
  it('formats zero', () => {
    expect(msToTimecode(0)).toBe('00:00:00')
  })

  it('formats seconds', () => {
    expect(msToTimecode(5000)).toBe('00:00:05')
  })

  it('formats minutes and seconds', () => {
    expect(msToTimecode(90000)).toBe('00:01:30')
  })

  it('formats hours', () => {
    expect(msToTimecode(3661000)).toBe('01:01:01')
  })

  it('truncates milliseconds', () => {
    expect(msToTimecode(5999)).toBe('00:00:05')
  })
})

describe('formatTranscriptForLLM', () => {
  it('formats segments with timecodes and milliseconds', () => {
    const result = formatTranscriptForLLM([
      { startMs: 0, endMs: 5000, text: 'Hello' },
      { startMs: 5000, endMs: 10000, text: 'World' }
    ])
    expect(result).toBe(
      '[#0 00:00:00 -> 00:00:05 | 0 -> 5000] Hello\n[#1 00:00:05 -> 00:00:10 | 5000 -> 10000] World'
    )
  })

  it('returns empty string for no segments', () => {
    expect(formatTranscriptForLLM([])).toBe('')
  })

  it('trims whitespace from text', () => {
    const result = formatTranscriptForLLM([
      { startMs: 0, endMs: 1000, text: '  spaced  ' }
    ])
    expect(result).toBe('[#0 00:00:00 -> 00:00:01 | 0 -> 1000] spaced')
  })
})

describe('parseLLMResponse', () => {
  it('parses a clean JSON array', () => {
    const text = JSON.stringify([
      { title: 'Clip 1', start_ms: 0, end_ms: 30000 },
      { title: 'Clip 2', start_ms: 60000, end_ms: 120000 }
    ])
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Clip 1', startMs: 0, endMs: 30000 },
      { title: 'Clip 2', startMs: 60000, endMs: 120000 }
    ])
  })

  it('extracts JSON from markdown code block', () => {
    const text = `Here are the clips:
\`\`\`json
[{"title": "Great moment", "start_ms": 1000, "end_ms": 5000}]
\`\`\``
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Great moment', startMs: 1000, endMs: 5000 }
    ])
  })

  it('extracts JSON with surrounding text', () => {
    const text = 'Based on the transcript, here are the best clips:\n[{"title": "Test", "start_ms": 0, "end_ms": 1000}]\nHope that helps!'
    const result = parseLLMResponse(text)
    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Test')
  })

  it('throws on missing JSON', () => {
    expect(() => parseLLMResponse('No clips found')).toThrow(
      'No JSON array found'
    )
  })

  it('parses start_id/end_id against transcript segments', () => {
    const segments = [
      { startMs: 0, endMs: 5000, text: 'A' },
      { startMs: 5000, endMs: 12000, text: 'B' },
      { startMs: 12000, endMs: 20000, text: 'C' }
    ]
    const text = JSON.stringify([
      { title: 'From ids', start_id: 1, end_id: 2 }
    ])
    const result = parseLLMResponse(text, segments)
    expect(result).toEqual([
      { title: 'From ids', startMs: 5000, endMs: 20000 }
    ])
  })

  it('coerces string millisecond values', () => {
    const text = JSON.stringify([
      { title: 'Strings', start_ms: '1000', end_ms: '5000' }
    ])
    const result = parseLLMResponse(text)
    expect(result[0].startMs).toBe(1000)
    expect(result[0].endMs).toBe(5000)
  })

  it('ignores a leading example array and parses the last JSON array', () => {
    const text = `Example: [{"title": "Example", "start_ms": 0, "end_ms": 1}]
Here are the clips:
[{"title": "Real", "start_ms": 4000, "end_ms": 9000}]`
    const result = parseLLMResponse(text)
    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('Real')
  })

  it('parses category field when present', () => {
    const text = JSON.stringify([
      { title: 'Main topic clip', start_ms: 0, end_ms: 60000, category: 'related' },
      { title: 'Self-contained insight', start_ms: 120000, end_ms: 180000, category: 'standalone' }
    ])
    const result = parseLLMResponse(text)
    expect(result).toEqual([
      { title: 'Main topic clip', startMs: 0, endMs: 60000, category: 'related' },
      { title: 'Self-contained insight', startMs: 120000, endMs: 180000, category: 'standalone' }
    ])
  })

  it('omits category when not present in response', () => {
    const text = JSON.stringify([
      { title: 'No category', start_ms: 0, end_ms: 30000 }
    ])
    const result = parseLLMResponse(text)
    expect(result[0].category).toBeUndefined()
  })

  it('ignores invalid category values', () => {
    const text = JSON.stringify([
      { title: 'Bad category', start_ms: 0, end_ms: 30000, category: 'invalid' }
    ])
    const result = parseLLMResponse(text)
    expect(result[0].category).toBeUndefined()
  })
})

// --- New tests ---

describe('parseTopicSegments', () => {
  it('parses valid JSON topic segments', () => {
    const text = JSON.stringify([
      { topic: 'Intro', start_ms: 0, end_ms: 60000, description: 'Introduction' },
      { topic: 'Main', start_ms: 60000, end_ms: 300000, description: 'Main content' }
    ])
    const result = parseTopicSegments(text)
    expect(result).toEqual([
      { topic: 'Intro', startMs: 0, endMs: 60000, description: 'Introduction' },
      { topic: 'Main', startMs: 60000, endMs: 300000, description: 'Main content' }
    ])
  })

  it('extracts JSON from markdown-wrapped response', () => {
    const text = `Here are the segments:\n\`\`\`json\n[{"topic": "A", "start_ms": 0, "end_ms": 100000, "description": "Desc"}]\n\`\`\``
    const result = parseTopicSegments(text)
    expect(result).toHaveLength(1)
    expect(result[0].topic).toBe('A')
  })

  it('throws on malformed response', () => {
    expect(() => parseTopicSegments('No JSON here')).toThrow('No JSON array found')
  })
})

describe('validateSegmentation', () => {
  const makeTopics = (ranges: [number, number][]): { topic: string; startMs: number; endMs: number; description: string }[] =>
    ranges.map(([s, e], i) => ({ topic: `T${i}`, startMs: s, endMs: e, description: '' }))

  it('returns true for good segmentation', () => {
    const topics = makeTopics([
      [0, 100000],
      [100000, 200000],
      [200000, 300000]
    ])
    expect(validateSegmentation(topics, 300000)).toBe(true)
  })

  it('returns false for fewer than 3 segments', () => {
    const topics = makeTopics([
      [0, 150000],
      [150000, 300000]
    ])
    expect(validateSegmentation(topics, 300000)).toBe(false)
  })

  it('returns false for low coverage', () => {
    const topics = makeTopics([
      [0, 50000],
      [50000, 100000],
      [100000, 130000]
    ])
    // Coverage = 130000 / 300000 ≈ 43%
    expect(validateSegmentation(topics, 300000)).toBe(false)
  })

  it('returns false for large gaps between segments', () => {
    const topics = makeTopics([
      [0, 100000],
      [110000, 200000], // 10s gap
      [200000, 300000]
    ])
    expect(validateSegmentation(topics, 300000)).toBe(false)
  })

  it('returns false for out of order segments', () => {
    const topics = makeTopics([
      [100000, 200000],
      [0, 100000], // starts before previous ends
      [200000, 300000]
    ])
    expect(validateSegmentation(topics, 300000)).toBe(false)
  })

  it('returns false for ultra-short segments', () => {
    const topics = makeTopics([
      [0, 10000], // 10s < 15s
      [10000, 200000],
      [200000, 300000]
    ])
    expect(validateSegmentation(topics, 300000)).toBe(false)
  })
})

describe('fallbackToTimeChunks', () => {
  const makeSegments = (count: number, spanMs: number) => {
    const segMs = spanMs / count
    return Array.from({ length: count }, (_, i) => ({
      startMs: Math.round(i * segMs),
      endMs: Math.round((i + 1) * segMs),
      text: `Segment ${i}`
    }))
  }

  it('covers full transcript with ~5min chunks', () => {
    const segments = makeSegments(60, 900000) // 15min video
    const chunks = fallbackToTimeChunks(segments)
    expect(chunks.length).toBe(3) // 15min / 5min = 3
    expect(chunks[0].startMs).toBe(0)
    expect(chunks[chunks.length - 1].endMs).toBe(900000)
  })

  it('handles short video (1 chunk)', () => {
    const segments = makeSegments(10, 120000) // 2min video
    const chunks = fallbackToTimeChunks(segments)
    expect(chunks.length).toBe(1)
    expect(chunks[0].startMs).toBe(0)
    expect(chunks[0].endMs).toBe(120000)
  })

  it('handles exact multiples', () => {
    const segments = makeSegments(30, 600000) // 10min = exactly 2 chunks
    const chunks = fallbackToTimeChunks(segments)
    expect(chunks.length).toBe(2)
    expect(chunks[0].endMs).toBe(300000)
    expect(chunks[1].endMs).toBe(600000)
  })

  it('returns empty array for empty segments', () => {
    expect(fallbackToTimeChunks([])).toEqual([])
  })
})

describe('getSegmentsInRange', () => {
  const segments = [
    { startMs: 0, endMs: 10000, text: 'A' },
    { startMs: 10000, endMs: 20000, text: 'B' },
    { startMs: 20000, endMs: 30000, text: 'C' },
    { startMs: 30000, endMs: 40000, text: 'D' },
    { startMs: 40000, endMs: 50000, text: 'E' }
  ]

  it('filters segments correctly within range', () => {
    const result = getSegmentsInRange(segments, 15000, 35000)
    expect(result.map((s) => s.text)).toEqual(['B', 'C', 'D'])
  })

  it('includes partial-overlap boundary segments', () => {
    const result = getSegmentsInRange(segments, 5000, 25000)
    // A overlaps (endMs 10000 > startMs 5000), B fully inside, C overlaps
    expect(result.map((s) => s.text)).toEqual(['A', 'B', 'C'])
  })

  it('returns empty for range outside segments', () => {
    const result = getSegmentsInRange(segments, 60000, 70000)
    expect(result).toEqual([])
  })
})

describe('generateTranscriptPreview', () => {
  const segments = [
    { startMs: 0, endMs: 10000, text: 'Line 1' },
    { startMs: 10000, endMs: 20000, text: 'Line 2' },
    { startMs: 20000, endMs: 30000, text: 'Line 3' },
    { startMs: 30000, endMs: 40000, text: 'Line 4' },
    { startMs: 40000, endMs: 50000, text: 'Line 5' },
    { startMs: 50000, endMs: 60000, text: 'Line 6' }
  ]

  it('returns first 2 + last 2 lines for a range with many segments', () => {
    const result = generateTranscriptPreview(segments, 0, 60000)
    expect(result).toBe('Line 1 | Line 2 | ... | Line 5 | Line 6')
  })

  it('handles ranges with fewer than 5 lines', () => {
    const result = generateTranscriptPreview(segments, 0, 30000)
    // 3 segments: Line 1, Line 2, Line 3 — no ellipsis
    expect(result).toBe('Line 1 | Line 2 | Line 3')
  })

  it('returns empty string for range with no segments', () => {
    const result = generateTranscriptPreview(segments, 70000, 80000)
    expect(result).toBe('')
  })

  it('handles exactly 4 lines without ellipsis', () => {
    const result = generateTranscriptPreview(segments, 0, 40000)
    expect(result).toBe('Line 1 | Line 2 | Line 3 | Line 4')
  })
})

describe('deduplicateClips', () => {
  const makeClip = (
    startMs: number,
    endMs: number,
    score: number,
    title: string = 'Clip'
  ) => ({
    title,
    startMs,
    endMs,
    score,
    justification: '',
    sourceTopic: 'T'
  })

  it('keeps all clips with no overlap', () => {
    const clips = [
      makeClip(0, 60000, 8),
      makeClip(120000, 180000, 7),
      makeClip(240000, 300000, 6)
    ]
    const result = deduplicateClips(clips)
    expect(result).toHaveLength(3)
  })

  it('keeps higher score when IoU > 0.5', () => {
    const clips = [
      makeClip(0, 60000, 5, 'Low'),
      makeClip(0, 60000, 9, 'High') // exact same range
    ]
    const result = deduplicateClips(clips)
    expect(result).toHaveLength(1)
    expect(result[0].title).toBe('High')
  })

  it('keeps both clips when IoU <= 0.5', () => {
    // A: 0-100000, B: 60000-200000
    // Intersection: 60000-100000 = 40000
    // Union: 0-200000 = 200000
    // IoU = 40000/200000 = 0.2
    const clips = [
      makeClip(0, 100000, 7),
      makeClip(60000, 200000, 8)
    ]
    const result = deduplicateClips(clips)
    expect(result).toHaveLength(2)
  })
})

describe('budgetCandidates', () => {
  const makeClip = (topic: string, score: number) => ({
    title: `Clip ${score}`,
    startMs: score * 10000,
    endMs: score * 10000 + 60000,
    score,
    justification: '',
    sourceTopic: topic
  })

  it('keeps top K per topic by score', () => {
    const candidates = [
      makeClip('A', 3),
      makeClip('A', 8),
      makeClip('A', 5),
      makeClip('A', 9),
      makeClip('A', 1),
      makeClip('A', 7) // 6 clips for topic A, keep top 2
    ]
    const result = budgetCandidates(candidates, 2)
    expect(result).toHaveLength(2)
    expect(result.map((c) => c.score).sort((a, b) => b - a)).toEqual([9, 8])
  })

  it('keeps all when fewer than K', () => {
    const candidates = [
      makeClip('A', 5),
      makeClip('A', 8)
    ]
    const result = budgetCandidates(candidates, 5)
    expect(result).toHaveLength(2)
  })

  it('budgets per topic independently', () => {
    const candidates = [
      makeClip('A', 9),
      makeClip('A', 7),
      makeClip('A', 3),
      makeClip('B', 8),
      makeClip('B', 4),
      makeClip('B', 2)
    ]
    const result = budgetCandidates(candidates, 2)
    expect(result).toHaveLength(4) // 2 from A + 2 from B
  })
})

describe('enforceCoverage', () => {
  const totalDurationMs = 900000 // 15min

  const makeCandidate = (startMs: number, endMs: number, score: number) => ({
    title: `Candidate ${startMs}`,
    startMs,
    endMs,
    score,
    justification: '',
    sourceTopic: 'T'
  })

  it('does not change when all thirds are covered', () => {
    const clips = [
      { title: 'A', startMs: 50000, endMs: 110000 },   // first third
      { title: 'B', startMs: 350000, endMs: 410000 },   // second third
      { title: 'C', startMs: 650000, endMs: 710000 }    // third third
    ]
    const candidates = [
      makeCandidate(50000, 110000, 8),
      makeCandidate(350000, 410000, 7),
      makeCandidate(650000, 710000, 6)
    ]
    const result = enforceCoverage(clips, candidates, totalDurationMs)
    expect(result).toHaveLength(3)
  })

  it('auto-fills missing third when candidates exist', () => {
    // Only clips in first and second third
    const clips = [
      { title: 'A', startMs: 50000, endMs: 110000 },
      { title: 'B', startMs: 350000, endMs: 410000 }
    ]
    const candidates = [
      makeCandidate(50000, 110000, 8),
      makeCandidate(350000, 410000, 7),
      makeCandidate(650000, 710000, 9),  // candidate in third third
      makeCandidate(700000, 760000, 6)
    ]
    const result = enforceCoverage(clips, candidates, totalDurationMs)
    expect(result).toHaveLength(3)
    // Should auto-fill from highest-scored candidate in the third third
    expect(result[2].startMs).toBe(650000)
  })

  it('does not change when missing third has no candidates', () => {
    const clips = [
      { title: 'A', startMs: 50000, endMs: 110000 },
      { title: 'B', startMs: 350000, endMs: 410000 }
    ]
    const candidates = [
      makeCandidate(50000, 110000, 8),
      makeCandidate(350000, 410000, 7)
      // No candidates in third third
    ]
    const result = enforceCoverage(clips, candidates, totalDurationMs)
    expect(result).toHaveLength(2) // unchanged
  })

  it('preserves sourceTopic as topic on auto-filled clips', () => {
    const clips = [
      { title: 'A', startMs: 50000, endMs: 110000 }
    ]
    const candidates = [
      { title: 'A', startMs: 50000, endMs: 110000, score: 8, justification: '', sourceTopic: 'Intro' },
      { title: 'Mid', startMs: 350000, endMs: 410000, score: 7, justification: '', sourceTopic: 'Main Content' },
      { title: 'End', startMs: 700000, endMs: 760000, score: 8, justification: '', sourceTopic: 'Conclusion' }
    ]
    const result = enforceCoverage(clips, candidates, totalDurationMs)
    expect(result).toHaveLength(3)
    // Auto-filled clips should carry topic from candidate's sourceTopic
    const midClip = result.find((c) => c.startMs === 350000)
    expect(midClip?.topic).toBe('Main Content')
    const endClip = result.find((c) => c.startMs === 700000)
    expect(endClip?.topic).toBe('Conclusion')
  })

  it('does not auto-fill a missing third with a weak candidate', () => {
    const clips = [
      { title: 'A', startMs: 50000, endMs: 110000 },
      { title: 'B', startMs: 350000, endMs: 410000 }
    ]
    const candidates = [
      makeCandidate(50000, 110000, 8),
      makeCandidate(350000, 410000, 7),
      makeCandidate(650000, 710000, 4)
    ]
    const result = enforceCoverage(clips, candidates, totalDurationMs)
    expect(result).toHaveLength(2)
  })
})

describe('extractJsonArray', () => {
  it('extracts a fenced json array', () => {
    const text = 'Sure.\n```json\n[{"a":1}]\n```\n'
    expect(extractJsonArray(text)).toEqual([{ a: 1 }])
  })

  it('uses bracket matching instead of a greedy first-to-last match', () => {
    const text = 'See [note] first. [{"title":"Ok","start_ms":1,"end_ms":2}] trailing [x]'
    expect(extractJsonArray(text)).toEqual([
      { title: 'Ok', start_ms: 1, end_ms: 2 }
    ])
  })

  it('unwraps { clips: [...] } objects', () => {
    expect(extractJsonArray('{"clips":[{"title":"A"}]}')).toEqual([
      { title: 'A' }
    ])
  })

  it('unwraps { items: [...] } objects', () => {
    expect(extractJsonArray('{"items":[1,2]}')).toEqual([1, 2])
  })
})

describe('analyzeTranscript hook pipeline', () => {
  const segments = Array.from({ length: 12 }, (_, i) => ({
    startMs: i * 5000,
    endMs: (i + 1) * 5000,
    text: `Line ${i}`
  }))

  it('resolves ranked candidate ids into snapped clip times', async () => {
    const llm = async (
      _provider: 'claude' | 'openai',
      _model: string,
      _apiKey: string,
      systemPrompt: string,
      _userMessage: string
    ): Promise<string> => {
      if (systemPrompt.includes('HOOK_FINDER')) {
        return JSON.stringify([{ hook_id: 6, score: 9, reason: 'the punchline' }])
      }
      if (systemPrompt.includes('CLIP_EXPANDER')) {
        return JSON.stringify([
          {
            title: 'The punchline',
            start_id: 5,
            end_id: 8,
            score: 9,
            justification: 'complete take'
          }
        ])
      }
      if (systemPrompt.includes('CLIP_RANKER')) {
        return JSON.stringify([
          { id: 1, title: 'The punchline', category: 'standalone' }
        ])
      }
      throw new Error(`unexpected prompt: ${systemPrompt.slice(0, 80)}`)
    }

    const { clips } = await analyzeTranscript(
      segments,
      'claude',
      'test-model',
      'key',
      undefined,
      undefined,
      llm
    )

    expect(clips.length).toBeGreaterThanOrEqual(1)
    expect(clips[0].title).toBe('The punchline')
    expect(clips[0].category).toBe('standalone')
    expect(clips[0].startMs).toBeLessThanOrEqual(segments[5].startMs)
    expect(clips[0].endMs).toBeGreaterThanOrEqual(segments[8].endMs)
  })

  it('falls back to time-chunk mining when hook finding returns nothing', async () => {
    const llm = async (
      _provider: 'claude' | 'openai',
      _model: string,
      _apiKey: string,
      systemPrompt: string
    ): Promise<string> => {
      if (systemPrompt.includes('HOOK_FINDER')) return '[]'
      if (systemPrompt.includes('CLIP_EXPANDER')) {
        return JSON.stringify([
          { title: 'Fallback clip', start_id: 0, end_id: 3, score: 8, justification: 'ok' }
        ])
      }
      if (systemPrompt.includes('CLIP_RANKER')) {
        return JSON.stringify([{ id: 1, title: 'Fallback clip', category: 'related' }])
      }
      return '[]'
    }

    const { clips } = await analyzeTranscript(
      segments,
      'claude',
      'test-model',
      'key',
      undefined,
      undefined,
      llm
    )

    expect(clips[0].title).toBe('Fallback clip')
    expect(clips[0].startMs).toBeLessThan(clips[0].endMs)
  })
})

describe('normalizeOpenCodeModel', () => {
  it('strips the opencode/ prefix', () => {
    expect(normalizeOpenCodeModel('opencode/minimax-m2.7')).toBe('minimax-m2.7')
  })

  it('leaves bare model ids unchanged', () => {
    expect(normalizeOpenCodeModel('claude-haiku-4-5')).toBe('claude-haiku-4-5')
  })
})

describe('openCodeApiKind', () => {
  it('routes Claude and Qwen models to the Anthropic messages API', () => {
    expect(openCodeApiKind('claude-haiku-4-5')).toBe('anthropic')
    expect(openCodeApiKind('opencode/claude-sonnet-4-5')).toBe('anthropic')
    expect(openCodeApiKind('qwen3.6-plus')).toBe('anthropic')
  })

  it('routes GPT, Grok, and Muse Spark models to the Responses API', () => {
    expect(openCodeApiKind('gpt-5.4-mini')).toBe('responses')
    expect(openCodeApiKind('grok-4.6')).toBe('responses')
    expect(openCodeApiKind('muse-spark-1.2')).toBe('responses')
  })

  it('routes Gemini models to generateContent', () => {
    expect(openCodeApiKind('gemini-3-flash')).toBe('gemini')
  })

  it('routes remaining models to chat completions', () => {
    expect(openCodeApiKind('minimax-m2.7')).toBe('chat')
    expect(openCodeApiKind('glm-5.1')).toBe('chat')
    expect(openCodeApiKind('kimi-k2.6')).toBe('chat')
    expect(openCodeApiKind('deepseek-v4-flash')).toBe('chat')
    expect(openCodeApiKind('big-pickle')).toBe('chat')
  })
})
