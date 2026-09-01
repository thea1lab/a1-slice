import Anthropic from '@anthropic-ai/sdk'
import OpenAI from 'openai'
import type {
  TranscriptSegment,
  ClipSegment,
  LLMProvider
} from '../shared/types'
import { OPENCODE_ZEN_BASE_URL } from '../shared/types'

export type OpenCodeApiKind = 'anthropic' | 'responses' | 'chat' | 'gemini'

/** Strip `opencode/` prefix used in OpenCode config model IDs. */
export function normalizeOpenCodeModel(model: string): string {
  return model.replace(/^opencode\//i, '').trim()
}

/**
 * OpenCode Zen uses different wire formats per model family.
 * See https://opencode.ai/docs/zen
 */
export function openCodeApiKind(model: string): OpenCodeApiKind {
  const id = normalizeOpenCodeModel(model).toLowerCase()
  if (id.startsWith('gemini-')) return 'gemini'
  if (id.startsWith('claude-') || id.startsWith('qwen')) return 'anthropic'
  if (
    id.startsWith('gpt-') ||
    id.startsWith('grok-') ||
    id.startsWith('muse-spark')
  ) {
    return 'responses'
  }
  return 'chat'
}

// --- Types ---

interface TopicSegment {
  topic: string
  startMs: number
  endMs: number
  description: string
}

interface CandidateClip {
  title: string
  startMs: number
  endMs: number
  score: number
  justification: string
  sourceTopic: string
}

// --- Utility functions ---

export function msToTimecode(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

export function formatTranscriptForLLM(segments: TranscriptSegment[]): string {
  return segments
    .map(
      (seg) =>
        `[${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)} | ${seg.startMs} -> ${seg.endMs}] ${seg.text.trim()}`
    )
    .join('\n')
}

export function parseLLMResponse(text: string): ClipSegment[] {
  // Extract JSON array from LLM response (may be wrapped in markdown code block)
  const jsonMatch = text.match(/\[[\s\S]*\]/)
  if (!jsonMatch) throw new Error('No JSON array found in LLM response')

  const parsed = JSON.parse(jsonMatch[0])
  if (!Array.isArray(parsed)) throw new Error('LLM response is not an array')

  return parsed.map(
    (item: { title: string; start_ms: number; end_ms: number; category?: string }) => {
      const clip: ClipSegment = {
        title: item.title,
        startMs: item.start_ms,
        endMs: item.end_ms
      }
      if (item.category === 'related' || item.category === 'standalone') {
        clip.category = item.category
      }
      return clip
    }
  )
}

// --- Helper functions for 3-phase pipeline ---

export function parseTopicSegments(text: string): TopicSegment[] {
  const jsonMatch = text.match(/\[[\s\S]*\]/)
  if (!jsonMatch) throw new Error('No JSON array found in topic segmentation response')

  const parsed = JSON.parse(jsonMatch[0])
  if (!Array.isArray(parsed)) throw new Error('Topic segmentation response is not an array')

  return parsed.map(
    (item: { topic: string; start_ms: number; end_ms: number; description: string }) => ({
      topic: item.topic,
      startMs: item.start_ms,
      endMs: item.end_ms,
      description: item.description
    })
  )
}

export function validateSegmentation(topics: TopicSegment[], totalDurationMs: number): boolean {
  if (topics.length < 3) return false

  // Check ordering — each segment must start at or after the previous one ends
  for (let i = 1; i < topics.length; i++) {
    if (topics[i].startMs < topics[i - 1].endMs - 1000) return false // allow 1s tolerance
  }

  // Check for ultra-short segments (<15s)
  for (const t of topics) {
    if (t.endMs - t.startMs < 15000) return false
  }

  // Check coverage — sum of segment durations should cover >=95% of total
  let coveredMs = 0
  for (const t of topics) {
    coveredMs += t.endMs - t.startMs
  }
  if (coveredMs < totalDurationMs * 0.95) return false

  // Check for large gaps (>5s between segments)
  for (let i = 1; i < topics.length; i++) {
    const gap = topics[i].startMs - topics[i - 1].endMs
    if (gap > 5000) return false
  }

  return true
}

export function fallbackToTimeChunks(
  segments: TranscriptSegment[],
  chunkMs: number = 300000
): TopicSegment[] {
  if (segments.length === 0) return []

  const totalStart = segments[0].startMs
  const totalEnd = segments[segments.length - 1].endMs
  const totalDuration = totalEnd - totalStart

  if (totalDuration <= 0) return []

  const chunks: TopicSegment[] = []
  let chunkStart = totalStart

  while (chunkStart < totalEnd) {
    const chunkEnd = Math.min(chunkStart + chunkMs, totalEnd)
    const chunkIndex = chunks.length + 1
    chunks.push({
      topic: `Section ${chunkIndex}`,
      startMs: chunkStart,
      endMs: chunkEnd,
      description: `Time chunk ${chunkIndex}`
    })
    chunkStart = chunkEnd
  }

  return chunks
}

export function getSegmentsInRange(
  segments: TranscriptSegment[],
  startMs: number,
  endMs: number
): TranscriptSegment[] {
  return segments.filter(
    (seg) => seg.endMs > startMs && seg.startMs < endMs
  )
}

export function generateTranscriptPreview(
  segments: TranscriptSegment[],
  startMs: number,
  endMs: number
): string {
  const inRange = getSegmentsInRange(segments, startMs, endMs)
  if (inRange.length === 0) return ''

  if (inRange.length <= 4) {
    return inRange.map((s) => s.text.trim()).join(' | ')
  }

  const first2 = inRange.slice(0, 2).map((s) => s.text.trim())
  const last2 = inRange.slice(-2).map((s) => s.text.trim())
  return [...first2, '...', ...last2].join(' | ')
}

export function deduplicateClips(clips: CandidateClip[]): CandidateClip[] {
  if (clips.length <= 1) return [...clips]

  // Sort by score descending so higher-scored clips are processed first
  const sorted = [...clips].sort((a, b) => b.score - a.score)
  const kept: CandidateClip[] = []

  for (const clip of sorted) {
    let dominated = false
    for (const existing of kept) {
      const iou = computeIoU(clip.startMs, clip.endMs, existing.startMs, existing.endMs)
      if (iou > 0.5) {
        // existing has higher or equal score (sorted order), so skip this clip
        dominated = true
        break
      }
    }
    if (!dominated) {
      kept.push(clip)
    }
  }

  return kept
}

function computeIoU(
  startA: number, endA: number,
  startB: number, endB: number
): number {
  const intersectionStart = Math.max(startA, startB)
  const intersectionEnd = Math.min(endA, endB)
  const intersection = Math.max(0, intersectionEnd - intersectionStart)

  const unionStart = Math.min(startA, startB)
  const unionEnd = Math.max(endA, endB)
  const union = unionEnd - unionStart

  if (union === 0) return 0
  return intersection / union
}

export function budgetCandidates(
  candidates: CandidateClip[],
  maxPerTopic: number = 5
): CandidateClip[] {
  const byTopic = new Map<string, CandidateClip[]>()
  for (const c of candidates) {
    const list = byTopic.get(c.sourceTopic) ?? []
    list.push(c)
    byTopic.set(c.sourceTopic, list)
  }

  const result: CandidateClip[] = []
  for (const [, topicClips] of byTopic) {
    const sorted = [...topicClips].sort((a, b) => b.score - a.score)
    result.push(...sorted.slice(0, maxPerTopic))
  }

  return result
}

export function enforceCoverage(
  finalClips: ClipSegment[],
  allCandidates: CandidateClip[],
  totalDurationMs: number
): ClipSegment[] {
  const thirdMs = totalDurationMs / 3
  const thirds = [
    { start: 0, end: thirdMs },
    { start: thirdMs, end: thirdMs * 2 },
    { start: thirdMs * 2, end: totalDurationMs }
  ]

  const result = [...finalClips]

  for (const third of thirds) {
    // Check if any final clip covers this third
    const hasCoverage = result.some(
      (c) => c.endMs > third.start && c.startMs < third.end
    )
    if (hasCoverage) continue

    // Find candidates in this third
    const candidatesInThird = allCandidates.filter(
      (c) => c.endMs > third.start && c.startMs < third.end
    )
    if (candidatesInThird.length === 0) continue

    // Pick highest-scored candidate
    const best = candidatesInThird.reduce((a, b) => (a.score >= b.score ? a : b))

    // Check we're not duplicating an existing clip
    const alreadyIncluded = result.some(
      (c) => computeIoU(c.startMs, c.endMs, best.startMs, best.endMs) > 0.5
    )
    if (!alreadyIncluded) {
      result.push({
        title: best.title,
        startMs: best.startMs,
        endMs: best.endMs,
        topic: best.sourceTopic
      })
    }
  }

  return result
}

// --- LLM call ---

async function callAnthropic(
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string,
  baseURL?: string
): Promise<string> {
  const client = new Anthropic({
    apiKey,
    ...(baseURL
      ? {
          baseURL,
          defaultHeaders: { Authorization: `Bearer ${apiKey}` }
        }
      : {})
  })
  const response = await client.messages.create({
    model,
    max_tokens: 4096,
    system: systemPrompt,
    messages: [{ role: 'user', content: userMessage }]
  })
  const block = response.content[0]
  return block.type === 'text' ? block.text : ''
}

async function callOpenAIChat(
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string,
  baseURL?: string
): Promise<string> {
  const client = new OpenAI({ apiKey, ...(baseURL ? { baseURL } : {}) })
  const response = await client.chat.completions.create({
    model,
    max_tokens: 4096,
    messages: [
      { role: 'system', content: systemPrompt },
      { role: 'user', content: userMessage }
    ]
  })
  return response.choices[0]?.message?.content ?? ''
}

function textFromResponses(response: {
  output_text?: string
  output?: Array<{
    type: string
    content?: Array<{ type: string; text?: string }>
  }>
}): string {
  if (typeof response.output_text === 'string' && response.output_text.length > 0) {
    return response.output_text
  }
  const parts: string[] = []
  for (const item of response.output ?? []) {
    if (item.type !== 'message' || !item.content) continue
    for (const block of item.content) {
      if (block.type === 'output_text' && block.text) parts.push(block.text)
    }
  }
  return parts.join('')
}

async function callOpenAIResponses(
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string,
  baseURL: string
): Promise<string> {
  const client = new OpenAI({ apiKey, baseURL })
  const response = await client.responses.create({
    model,
    max_output_tokens: 4096,
    instructions: systemPrompt,
    input: userMessage
  })
  return textFromResponses(response)
}

async function callOpenCodeGemini(
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string
): Promise<string> {
  const url = `${OPENCODE_ZEN_BASE_URL}/models/${encodeURIComponent(model)}:generateContent`
  const res = await fetch(url, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${apiKey}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      system_instruction: { parts: [{ text: systemPrompt }] },
      contents: [{ role: 'user', parts: [{ text: userMessage }] }],
      generationConfig: { maxOutputTokens: 4096 }
    })
  })
  const data = (await res.json()) as {
    candidates?: Array<{ content?: { parts?: Array<{ text?: string }> } }>
    error?: { message?: string }
  }
  if (!res.ok) {
    throw new Error(data.error?.message || `OpenCode Gemini request failed (${res.status})`)
  }
  return (
    data.candidates?.[0]?.content?.parts
      ?.map((part) => part.text ?? '')
      .join('') ?? ''
  )
}

async function callOpenCode(
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string
): Promise<string> {
  const id = normalizeOpenCodeModel(model)
  const kind = openCodeApiKind(id)
  if (kind === 'anthropic') {
    return callAnthropic(
      id,
      apiKey,
      systemPrompt,
      userMessage,
      'https://opencode.ai/zen'
    )
  }
  if (kind === 'responses') {
    return callOpenAIResponses(
      id,
      apiKey,
      systemPrompt,
      userMessage,
      OPENCODE_ZEN_BASE_URL
    )
  }
  if (kind === 'gemini') {
    return callOpenCodeGemini(id, apiKey, systemPrompt, userMessage)
  }
  return callOpenAIChat(id, apiKey, systemPrompt, userMessage, OPENCODE_ZEN_BASE_URL)
}

async function callLLM(
  provider: LLMProvider,
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string
): Promise<string> {
  if (provider === 'claude') {
    return callAnthropic(model, apiKey, systemPrompt, userMessage)
  }
  if (provider === 'opencode') {
    return callOpenCode(model, apiKey, systemPrompt, userMessage)
  }
  return callOpenAIChat(model, apiKey, systemPrompt, userMessage)
}

// --- Prompts ---

const SEGMENTATION_SYSTEM_PROMPT = `You are a video content analyst. You will receive a timestamped transcript of a video. Your job is to divide the transcript into 5-15 contiguous topic segments that cover the ENTIRE video.

RULES:
- Each segment must have a clear topic or theme
- Segments must be contiguous — no gaps allowed. The end of one segment must equal the start of the next.
- The first segment must start at the beginning of the transcript
- The last segment must end at the end of the transcript
- Each segment should be at least 15 seconds long

TIMESTAMPS:
- The transcript includes millisecond values after the pipe (|) character. Use these exact values for start_ms and end_ms. Do NOT convert or calculate — just copy the numbers directly.

Return a JSON array (no other text) where each element has:
- "topic": a short label for the segment's topic
- "start_ms": start time in milliseconds
- "end_ms": end time in milliseconds
- "description": a 1-2 sentence description of what happens in this segment

Example:
[
  {"topic": "Introduction", "start_ms": 0, "end_ms": 45000, "description": "The host introduces themselves and the topic of today's video."},
  {"topic": "Main argument", "start_ms": 45000, "end_ms": 180000, "description": "A detailed walkthrough of the core thesis with examples."}
]`

const CHUNK_CLIP_SYSTEM_PROMPT = `You are a video editor AI. You will receive a section of a video transcript. Your job is to find ALL clip-worthy moments in this section.

Lines marked with [CONTEXT] at the beginning are boundary context from adjacent sections — they help you understand transitions but clips should NOT start or end within context lines.

CLIP GUIDELINES:
- Target duration: 30-90 seconds per clip. ~60 seconds is ideal.
- Each clip must have clean start and end points (not mid-sentence).
- Each clip must tell a complete story or make a complete point.
- Find ALL worthy moments — don't limit yourself.

SCORING:
- Assign each clip a score from 1 to 10 (integer) reflecting standalone clip quality:
  - 9-10: Exceptional — viral potential, powerful insight, or peak entertainment
  - 7-8: Strong — valuable content that stands well on its own
  - 5-6: Good — decent content worth considering
  - 3-4: Marginal — only worth including if few better options exist
  - 1-2: Weak — filler or low-value content

TIMESTAMPS:
- The transcript includes millisecond values after the pipe (|) character. Use these exact values for start_ms and end_ms.

Return a JSON array (no other text) where each element has:
- "title": a short, catchy title for the clip
- "start_ms": start time in milliseconds
- "end_ms": end time in milliseconds
- "score": integer 1-10 quality score
- "justification": one sentence explaining why this is clip-worthy

If no clips are worth extracting from this section, return an empty array: []`

const CLIP_SYSTEM_PROMPT = `You are a video editor AI. You will receive:
1. A structured analysis of a video (from a previous analysis phase)
2. The original timestamped transcript

Your job is to select the best clips from this video for social media.

CLIP GUIDELINES:
- Target duration: 30-90 seconds per clip. ~60 seconds is ideal.
- Identify ALL high-value content worth clipping. Don't limit yourself to just 1-2 clips — if there are 5 great moments, return 5 clips.
- If a great section is longer than 90 seconds, split it into multiple clips.
- Each clip must have clean start and end points (not mid-sentence).
- Each clip must tell a complete story or make a complete point.

STRATEGY:
- Use the KEY MOMENTS and HIGH-value sections from the analysis as your primary sources.
- Avoid DEAD ZONES identified in the analysis.
- Prioritize sections with standalone potential.

CATEGORIES — assign each clip one of:
- "related": This clip is about the video's main topic. It works well for promoting the full video.
- "standalone": This clip delivers value entirely on its own. It works without any context about the source video.

TIMESTAMPS:
- The transcript includes millisecond values after the pipe (|) character. Use these exact values for start_ms and end_ms. Do NOT convert or calculate — just copy the numbers directly.

Return a JSON array (no other text) where each element has:
- "title": a short, catchy title for the clip
- "start_ms": start time in milliseconds (must match a value from the transcript)
- "end_ms": end time in milliseconds (must match a value from the transcript)
- "category": either "related" or "standalone"

Example:
[
  {"title": "The moment everything changed", "start_ms": 45000, "end_ms": 105000, "category": "standalone"},
  {"title": "Best advice for beginners", "start_ms": 300000, "end_ms": 360000, "category": "related"}
]`

const RANKING_SYSTEM_PROMPT = `You are a video editor AI. You will receive a list of candidate clips extracted from different sections of a video. Each candidate has a title, time range, quality score, justification, and a transcript preview.

Your job is to rank, deduplicate, and select the best clips for final output.

RULES:
- Remove redundant clips that cover the same content
- Ensure clips spread across the FULL video timeline — do not cluster clips in one section
- Assign each clip a category:
  - "related": This clip is about the video's main topic. It works well for promoting the full video.
  - "standalone": This clip delivers value entirely on its own. It works without any context about the source video.
- Prefer clips with higher scores, but balance quality with timeline coverage
- Return 3-10 clips depending on video length and content quality

Return a JSON array (no other text) where each element has:
- "title": the clip title (may be refined from the candidate title)
- "start_ms": start time in milliseconds
- "end_ms": end time in milliseconds
- "category": either "related" or "standalone"

Example:
[
  {"title": "The key insight", "start_ms": 45000, "end_ms": 105000, "category": "standalone"},
  {"title": "Practical advice", "start_ms": 300000, "end_ms": 360000, "category": "related"}
]`

// --- Concurrency helper ---

function createSemaphore(maxConcurrent: number) {
  let running = 0
  const queue: (() => void)[] = []

  return {
    async acquire(): Promise<void> {
      if (running < maxConcurrent) {
        running++
        return
      }
      await new Promise<void>((resolve) => queue.push(resolve))
    },
    release(): void {
      running--
      const next = queue.shift()
      if (next) {
        running++
        next()
      }
    }
  }
}

// --- Phase 2 response parser ---

function parseCandidateClips(text: string, sourceTopic: string): CandidateClip[] {
  const jsonMatch = text.match(/\[[\s\S]*\]/)
  if (!jsonMatch) return []

  try {
    const parsed = JSON.parse(jsonMatch[0])
    if (!Array.isArray(parsed)) return []

    return parsed
      .filter(
        (item: any) =>
          item.title && typeof item.start_ms === 'number' && typeof item.end_ms === 'number'
      )
      .map(
        (item: {
          title: string
          start_ms: number
          end_ms: number
          score?: number
          justification?: string
        }) => ({
          title: item.title,
          startMs: item.start_ms,
          endMs: item.end_ms,
          score: typeof item.score === 'number' ? Math.max(1, Math.min(10, Math.round(item.score))) : 5,
          justification: item.justification ?? '',
          sourceTopic
        })
      )
  } catch {
    return []
  }
}

// --- Main pipeline ---

export async function analyzeTranscript(
  segments: TranscriptSegment[],
  provider: LLMProvider,
  model: string,
  apiKey: string,
  userHint?: string,
  onProgress?: (message: string, percent: number) => void
): Promise<{ clips: ClipSegment[]; rawResponse: string }> {
  const formattedTranscript = formatTranscriptForLLM(segments)
  const totalDurationMs =
    segments.length > 0 ? segments[segments.length - 1].endMs - segments[0].startMs : 0

  let debugOutput = ''

  // --- Phase 1: Topic Segmentation ---
  onProgress?.('Identifying video topics...', 5)

  let segmentationMessage = `Here is the transcript:\n\n${formattedTranscript}\n\nDivide this video into topic segments.`
  if (userHint?.trim()) {
    segmentationMessage += `\n\nAdditional context from the user: ${userHint.trim()}`
  }

  const segmentationResponse = await callLLM(
    provider, model, apiKey,
    SEGMENTATION_SYSTEM_PROMPT, segmentationMessage
  )

  debugOutput += `=== PHASE 1: TOPIC SEGMENTATION ===\n${segmentationResponse}\n\n`

  let topics: TopicSegment[]
  try {
    topics = parseTopicSegments(segmentationResponse)
    if (!validateSegmentation(topics, totalDurationMs)) {
      topics = fallbackToTimeChunks(segments)
      debugOutput += `[Segmentation validation failed — using time chunks]\n\n`
    }
  } catch {
    topics = fallbackToTimeChunks(segments)
    debugOutput += `[Segmentation parse failed — using time chunks]\n\n`
  }

  onProgress?.('Identifying video topics...', 20)

  // --- Phase 2: Per-Topic Clip Finding ---
  const semaphore = createSemaphore(3)
  let allCandidates: CandidateClip[] = []
  let completedChunks = 0

  const chunkTasks = topics.map((topic, index) => async () => {
    await semaphore.acquire()
    try {
      // Get transcript for this topic with ±30s context
      const contextStartMs = topic.startMs - 30000
      const contextEndMs = topic.endMs + 30000
      const topicSegments = getSegmentsInRange(segments, contextStartMs, contextEndMs)

      // Format with [CONTEXT] markers for boundary lines
      const formatted = topicSegments
        .map((seg) => {
          const isContext = seg.endMs <= topic.startMs || seg.startMs >= topic.endMs
          const prefix = isContext ? '[CONTEXT] ' : ''
          return `${prefix}[${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)} | ${seg.startMs} -> ${seg.endMs}] ${seg.text.trim()}`
        })
        .join('\n')

      let userMessage = `Section: ${topic.topic}. ${topic.description}\n\n${formatted}\n\nFind all clip-worthy moments in this section.`
      if (userHint?.trim()) {
        userMessage += `\n\nUser preferences: ${userHint.trim()}`
      }

      const response = await callLLM(
        provider, model, apiKey,
        CHUNK_CLIP_SYSTEM_PROMPT, userMessage
      )

      debugOutput += `=== PHASE 2: TOPIC ${index + 1} — ${topic.topic} ===\n${response}\n\n`

      return parseCandidateClips(response, topic.topic)
    } finally {
      semaphore.release()
      completedChunks++
      const chunkPercent = 20 + Math.round((completedChunks / topics.length) * 55)
      onProgress?.(`Finding clips in: ${topic.topic}...`, chunkPercent)
    }
  })

  const results = await Promise.allSettled(chunkTasks.map((task) => task()))

  // Collect results, retry failed ones once
  const failedIndices: number[] = []
  for (let i = 0; i < results.length; i++) {
    const result = results[i]
    if (result.status === 'fulfilled' && result.value.length > 0) {
      allCandidates.push(...result.value)
    } else if (result.status === 'rejected') {
      failedIndices.push(i)
    }
  }

  // Retry failed chunks once with 1s backoff
  if (failedIndices.length > 0) {
    await new Promise((r) => setTimeout(r, 1000))
    const retryResults = await Promise.allSettled(
      failedIndices.map((idx) => chunkTasks[idx]())
    )
    for (const result of retryResults) {
      if (result.status === 'fulfilled' && result.value.length > 0) {
        allCandidates.push(...result.value)
      }
    }
  }

  // --- Global Rescue Fallback ---
  if (allCandidates.length === 0) {
    debugOutput += `=== GLOBAL RESCUE (all chunks produced no candidates) ===\n`

    let rescueMessage = `## ORIGINAL TRANSCRIPT\n\n${formattedTranscript}\n\nSelect the best clips from this video. Return only a JSON array.`
    if (userHint?.trim()) {
      rescueMessage += `\n\nUser preferences: ${userHint.trim()}`
    }

    const rescueResponse = await callLLM(
      provider, model, apiKey,
      CLIP_SYSTEM_PROMPT, rescueMessage
    )
    debugOutput += `${rescueResponse}\n\n`
    onProgress?.('Ranking and selecting best clips...', 90)

    const clips = parseLLMResponse(rescueResponse)
    debugOutput += `=== FINAL OUTPUT (rescue) ===\n${rescueResponse}\n`
    return { clips, rawResponse: debugOutput }
  }

  // Budget candidates before Phase 3
  allCandidates = budgetCandidates(allCandidates, 5)

  // --- Phase 3: Global Ranking ---
  onProgress?.('Ranking and selecting best clips...', 75)

  // Build numbered candidate list with transcript previews
  const candidateList = allCandidates
    .map((c, i) => {
      const preview = generateTranscriptPreview(segments, c.startMs, c.endMs)
      return `${i + 1}. "${c.title}" [${msToTimecode(c.startMs)} - ${msToTimecode(c.endMs)}] (${c.startMs}-${c.endMs}ms) Score: ${c.score}/10\n   Topic: ${c.sourceTopic}\n   Why: ${c.justification}\n   Preview: ${preview}`
    })
    .join('\n\n')

  const rankingMessage = `Here are ${allCandidates.length} candidate clips from the video:\n\n${candidateList}\n\nRank and select the best clips. Ensure clips spread across the full video timeline. Return only a JSON array.`

  const rankingResponse = await callLLM(
    provider, model, apiKey,
    RANKING_SYSTEM_PROMPT, rankingMessage
  )

  debugOutput += `=== PHASE 3: GLOBAL RANKING ===\n${rankingResponse}\n\n`

  onProgress?.('Processing results...', 95)

  let finalClips: ClipSegment[]
  try {
    finalClips = parseLLMResponse(rankingResponse)
  } catch {
    // If ranking parse fails, convert top candidates directly
    finalClips = allCandidates
      .sort((a, b) => b.score - a.score)
      .slice(0, 8)
      .map((c) => ({ title: c.title, startMs: c.startMs, endMs: c.endMs }))
  }

  // Deduplicate using IoU on candidate scores
  const candidateMap = new Map<string, CandidateClip>()
  for (const c of allCandidates) {
    const key = `${c.startMs}-${c.endMs}`
    candidateMap.set(key, c)
  }

  const asDedup: CandidateClip[] = finalClips.map((c) => {
    const match = candidateMap.get(`${c.startMs}-${c.endMs}`)
    return {
      title: c.title,
      startMs: c.startMs,
      endMs: c.endMs,
      score: match?.score ?? 5,
      justification: match?.justification ?? '',
      sourceTopic: match?.sourceTopic ?? ''
    }
  })

  const deduped = deduplicateClips(asDedup)
  finalClips = deduped.map((c) => {
    const original = finalClips.find(
      (f) => f.startMs === c.startMs && f.endMs === c.endMs
    )
    const match = candidateMap.get(`${c.startMs}-${c.endMs}`)
    return {
      title: c.title,
      startMs: c.startMs,
      endMs: c.endMs,
      category: original?.category,
      topic: match?.sourceTopic
    }
  })

  // Enforce coverage
  finalClips = enforceCoverage(finalClips, allCandidates, totalDurationMs)

  const finalJson = JSON.stringify(
    finalClips.map((c) => ({
      title: c.title,
      start_ms: c.startMs,
      end_ms: c.endMs,
      category: c.category,
      topic: c.topic
    })),
    null,
    2
  )
  debugOutput += `=== FINAL OUTPUT ===\n${finalJson}\n`

  return { clips: finalClips, rawResponse: debugOutput }
}
