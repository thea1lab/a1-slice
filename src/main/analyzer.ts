import Anthropic from '@anthropic-ai/sdk'
import OpenAI from 'openai'
import type {
  TranscriptSegment,
  ClipSegment,
  LLMProvider
} from '../shared/types'
import { refineClipBounds } from '../shared/clipBounds'

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

export function formatTranscriptForLLM(
  segments: TranscriptSegment[],
  startIndex = 0
): string {
  return segments
    .map(
      (seg, i) =>
        `[#${startIndex + i} ${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)} | ${seg.startMs} -> ${seg.endMs}] ${seg.text.trim()}`
    )
    .join('\n')
}

function extractBalanced(text: string, start: number): string | null {
  const open = text[start]
  const close = open === '[' ? ']' : '}'
  let depth = 0
  let inString = false
  let escape = false
  for (let i = start; i < text.length; i++) {
    const ch = text[i]
    if (inString) {
      if (escape) {
        escape = false
        continue
      }
      if (ch === '\\') {
        escape = true
        continue
      }
      if (ch === '"') inString = false
      continue
    }
    if (ch === '"') {
      inString = true
      continue
    }
    if (ch === open) depth++
    else if (ch === close) {
      depth--
      if (depth === 0) return text.slice(start, i + 1)
    }
  }
  return null
}

function unwrapJsonArray(value: unknown): unknown[] | null {
  if (Array.isArray(value)) return value
  if (value && typeof value === 'object') {
    const obj = value as Record<string, unknown>
    for (const key of ['clips', 'items', 'hooks', 'candidates', 'segments', 'topics']) {
      if (Array.isArray(obj[key])) return obj[key]
    }
    const arrayVals = Object.values(obj).filter(Array.isArray)
    if (arrayVals.length === 1) return arrayVals[0] as unknown[]
  }
  return null
}

export function extractJsonArray(text: string): unknown[] {
  const fence = text.match(/```(?:json)?\s*([\s\S]*?)```/i)
  const sources = fence ? [fence[1].trim(), text] : [text]
  const arrays: unknown[][] = []

  for (const src of sources) {
    for (let i = 0; i < src.length; i++) {
      if (src[i] !== '[' && src[i] !== '{') continue
      const slice = extractBalanced(src, i)
      if (!slice) continue
      try {
        const parsed = JSON.parse(slice)
        const arr = unwrapJsonArray(parsed)
        if (arr) {
          arrays.push(arr)
          i += slice.length - 1
        }
      } catch {
        // keep scanning for the next balanced value
      }
    }
    if (arrays.length > 0) break
  }

  if (arrays.length === 0) throw new Error('No JSON array found in LLM response')
  return arrays[arrays.length - 1]
}

function coerceNumber(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string' && value.trim() !== '') {
    const n = Number(value)
    if (Number.isFinite(n)) return n
  }
  return null
}

function clipFromParsedItem(
  item: Record<string, unknown>,
  segments?: TranscriptSegment[]
): ClipSegment | null {
  const title = typeof item.title === 'string' && item.title.trim() ? item.title : 'Clip'
  const startId = coerceNumber(item.start_id)
  const endId = coerceNumber(item.end_id)
  let startMs: number | null = null
  let endMs: number | null = null

  if (
    segments &&
    startId != null &&
    endId != null &&
    segments[startId] &&
    segments[endId]
  ) {
    startMs = segments[startId].startMs
    endMs = segments[endId].endMs
  } else {
    startMs = coerceNumber(item.start_ms)
    endMs = coerceNumber(item.end_ms)
  }

  if (startMs == null || endMs == null) return null
  if (endMs < startMs) {
    const tmp = startMs
    startMs = endMs
    endMs = tmp
  }

  const clip: ClipSegment = { title, startMs, endMs }
  if (item.category === 'related' || item.category === 'standalone') {
    clip.category = item.category
  }
  return clip
}

export function parseLLMResponse(text: string, segments?: TranscriptSegment[]): ClipSegment[] {
  const parsed = extractJsonArray(text)
  return parsed
    .map((item) =>
      item && typeof item === 'object'
        ? clipFromParsedItem(item as Record<string, unknown>, segments)
        : null
    )
    .filter((clip): clip is ClipSegment => clip != null)
}

// --- Helper functions for 3-phase pipeline ---

export function parseTopicSegments(text: string): TopicSegment[] {
  const parsed = extractJsonArray(text)

  return parsed.map((raw) => {
    const item = raw as { topic: string; start_ms: number; end_ms: number; description: string }
    return {
      topic: item.topic,
      startMs: coerceNumber(item.start_ms) ?? 0,
      endMs: coerceNumber(item.end_ms) ?? 0,
      description: item.description
    }
  })
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
  totalDurationMs: number,
  minScore: number = 7
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
      (c) => c.score >= minScore && c.endMs > third.start && c.startMs < third.end
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

async function callLLM(
  provider: LLMProvider,
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string
): Promise<string> {
  if (provider === 'claude') {
    const client = new Anthropic({ apiKey })
    const response = await client.messages.create({
      model,
      max_tokens: 4096,
      system: systemPrompt,
      messages: [{ role: 'user', content: userMessage }]
    })
    const block = response.content[0]
    return block.type === 'text' ? block.text : ''
  } else {
    const client = new OpenAI({ apiKey })
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
}

export type LLMCaller = (
  provider: LLMProvider,
  model: string,
  apiKey: string,
  systemPrompt: string,
  userMessage: string
) => Promise<string>

// --- Prompts ---

const HOOK_SYSTEM_PROMPT = `HOOK_FINDER. You are a video editor looking for hot points — the line you would put on a thumbnail.

You will receive a numbered transcript. Each line starts with [#N ...]. N is the segment id.

Find 8-15 peak moments: surprise, a strong claim, a punchline, a concrete tip, or an emotional turn.
Do NOT pick greetings, filler, or "and then they discuss...".
Do NOT try to cover the whole video. Only true peaks.

Return a JSON array (no other text):
[{"hook_id": 12, "score": 9, "reason": "one sentence why this line is the hook"}]

hook_id MUST be a segment id from the transcript. If nothing is clip-worthy, return [].`

const CLIP_EXPANDER_PROMPT = `CLIP_EXPANDER. You are a video editor. You will receive a numbered transcript window. Each line starts with [#N ...]. N is the segment id.

Your job is to turn a hot moment into a complete social clip.

RULES:
- Return start_id and end_id using the #N ids. Never invent milliseconds. Never convert timecodes.
- Lines marked [CONTEXT] are adjacent material — do not start or end a clip on a CONTEXT line.
- Duration: 20-90 seconds. ~45-60 seconds is ideal. Shorter punchlines (about 20s) are allowed.
- Start on a complete thought, not mid-sentence. End after the payoff lands.
- If a hook line is given, the clip MUST include that hook.
- Skip greetings, setup-only, and "in this video we will..." material.

SCORING (integer 1-10):
- 9-10: exceptional hook + payoff
- 7-8: strong standalone clip
- 5-6: decent
- 1-4: weak / filler

Return a JSON array (no other text):
[{"title": "short catchy title", "start_id": 10, "end_id": 18, "score": 8, "justification": "one sentence"}]

If nothing is worth clipping, return [].`

const CLIP_SYSTEM_PROMPT = `You are a video editor AI. You will receive a numbered transcript. Each line starts with [#N ...]. N is the segment id.

Select the best social clips from this video.

RULES:
- Target duration 20-90 seconds. ~45-60 seconds is ideal.
- Each clip must be a complete thought with a hook and a payoff.
- Use start_id and end_id from the #N ids. Do NOT invent milliseconds. Do NOT convert timecodes.
- Skip filler, greetings, and weak coverage-for-coverage's-sake moments.

CATEGORIES:
- "related": about the video's main topic, useful to promote the full video
- "standalone": valuable with no extra context

Return a JSON array (no other text):
[{"title": "short catchy title", "start_id": 10, "end_id": 18, "category": "standalone"}]

If nothing is worth clipping, return [].`

const RANKING_SYSTEM_PROMPT = `CLIP_RANKER. You will receive numbered candidate clips with full transcript text, scores, and reasons.

Select the best final clips.

RULES:
- Choose candidates by id only. You may change title and category. Do NOT change timestamps. Do NOT invent new clips.
- Remove redundant clips that cover the same take.
- Prefer higher scores. Do not force weak clips just to spread across the timeline.
- Return 3-10 clips depending on how many are actually good. If only 1-2 are great, return those.

CATEGORIES:
- "related": about the video's main topic
- "standalone": valuable on its own

Return a JSON array (no other text):
[{"id": 1, "title": "refined title", "category": "standalone"}]`

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

function parseCandidateClips(
  text: string,
  sourceTopic: string,
  segments?: TranscriptSegment[]
): CandidateClip[] {
  try {
    const parsed = extractJsonArray(text)
    return parsed
      .map((raw) => {
        if (!raw || typeof raw !== 'object') return null
        const clip = clipFromParsedItem(raw as Record<string, unknown>, segments)
        if (!clip) return null
        const item = raw as { score?: unknown; justification?: unknown; title?: unknown }
        return {
          title: clip.title,
          startMs: clip.startMs,
          endMs: clip.endMs,
          score: (() => {
            const n = coerceNumber(item.score)
            return n == null ? 5 : Math.max(1, Math.min(10, Math.round(n)))
          })(),
          justification: typeof item.justification === 'string' ? item.justification : '',
          sourceTopic
        }
      })
      .filter((c): c is CandidateClip => c != null)
  } catch {
    return []
  }
}

function formatWindow(
  segments: TranscriptSegment[],
  windowStartMs: number,
  windowEndMs: number,
  core?: { startMs: number; endMs: number }
): string {
  return segments
    .map((seg, index) => ({ seg, index }))
    .filter(({ seg }) => seg.endMs > windowStartMs && seg.startMs < windowEndMs)
    .map(({ seg, index }) => {
      const isContext = core
        ? seg.endMs <= core.startMs || seg.startMs >= core.endMs
        : false
      const prefix = isContext ? '[CONTEXT] ' : ''
      return `${prefix}[#${index} ${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)} | ${seg.startMs} -> ${seg.endMs}] ${seg.text.trim()}`
    })
    .join('\n')
}

function parseHooks(
  text: string,
  segmentCount: number
): { hookId: number; score: number; reason: string }[] {
  try {
    const parsed = extractJsonArray(text)
    return parsed
      .map((raw) => {
        if (!raw || typeof raw !== 'object') return null
        const item = raw as Record<string, unknown>
        const hookId = coerceNumber(item.hook_id)
        if (hookId == null || hookId < 0 || hookId >= segmentCount) return null
        const score = coerceNumber(item.score)
        return {
          hookId,
          score: score == null ? 5 : Math.max(1, Math.min(10, Math.round(score))),
          reason: typeof item.reason === 'string' ? item.reason : ''
        }
      })
      .filter((h): h is { hookId: number; score: number; reason: string } => h != null)
  } catch {
    return []
  }
}

function parseRankedIds(
  text: string
): { id: number; title?: string; category?: 'related' | 'standalone' }[] {
  try {
    const parsed = extractJsonArray(text)
    const rows: { id: number; title?: string; category?: 'related' | 'standalone' }[] = []
    for (const raw of parsed) {
      if (!raw || typeof raw !== 'object') continue
      const item = raw as Record<string, unknown>
      const id = coerceNumber(item.id) ?? coerceNumber(item.candidate_id)
      if (id == null) continue
      const row: { id: number; title?: string; category?: 'related' | 'standalone' } = { id }
      if (typeof item.title === 'string') row.title = item.title
      if (item.category === 'related' || item.category === 'standalone') {
        row.category = item.category
      }
      rows.push(row)
    }
    return rows
  } catch {
    return []
  }
}

interface MineWindow {
  label: string
  userMessage: string
}

async function mineWindows(
  windows: MineWindow[],
  segments: TranscriptSegment[],
  provider: LLMProvider,
  model: string,
  apiKey: string,
  llmCall: LLMCaller,
  onProgress?: (message: string, percent: number) => void,
  debugSink?: (chunk: string) => void
): Promise<CandidateClip[]> {
  if (windows.length === 0) return []

  const semaphore = createSemaphore(3)
  let completed = 0
  const tasks = windows.map((window, index) => async () => {
    await semaphore.acquire()
    try {
      const response = await llmCall(
        provider,
        model,
        apiKey,
        CLIP_EXPANDER_PROMPT,
        window.userMessage
      )
      debugSink?.(`=== PHASE 2: ${index + 1} — ${window.label} ===\n${response}\n\n`)
      return parseCandidateClips(response, window.label, segments)
    } finally {
      semaphore.release()
      completed++
      const percent = 20 + Math.round((completed / windows.length) * 55)
      onProgress?.(`Finding clips in: ${window.label}...`, percent)
    }
  })

  const results = await Promise.allSettled(tasks.map((task) => task()))
  const candidates: CandidateClip[] = []
  const failedIndices: number[] = []
  for (let i = 0; i < results.length; i++) {
    const result = results[i]
    if (result.status === 'fulfilled' && result.value.length > 0) {
      candidates.push(...result.value)
    } else if (result.status === 'rejected') {
      failedIndices.push(i)
    }
  }

  if (failedIndices.length > 0) {
    await new Promise((r) => setTimeout(r, 1000))
    const retryResults = await Promise.allSettled(failedIndices.map((idx) => tasks[idx]()))
    for (const result of retryResults) {
      if (result.status === 'fulfilled' && result.value.length > 0) {
        candidates.push(...result.value)
      }
    }
  }

  return candidates
}

function assembleFinalClips(
  finalClips: ClipSegment[],
  allCandidates: CandidateClip[],
  segments: TranscriptSegment[],
  totalDurationMs: number
): ClipSegment[] {
  const candidateMap = new Map<string, CandidateClip>()
  for (const c of allCandidates) {
    candidateMap.set(`${c.startMs}-${c.endMs}`, c)
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
  let clips: ClipSegment[] = deduped.map((c) => {
    const original = finalClips.find((f) => f.startMs === c.startMs && f.endMs === c.endMs)
    const match = candidateMap.get(`${c.startMs}-${c.endMs}`)
    return refineClipBounds(
      {
        title: c.title,
        startMs: c.startMs,
        endMs: c.endMs,
        category: original?.category,
        topic: match?.sourceTopic
      },
      segments
    )
  })

  clips = enforceCoverage(clips, allCandidates, totalDurationMs, 7)
  clips = clips
    .map((c) => refineClipBounds(c, segments))
    .filter((c) => c.endMs > c.startMs)

  const afterSnap: CandidateClip[] = clips.map((c) => ({
    title: c.title,
    startMs: c.startMs,
    endMs: c.endMs,
    score: candidateMap.get(`${c.startMs}-${c.endMs}`)?.score ?? 5,
    justification: '',
    sourceTopic: c.topic ?? ''
  }))
  return deduplicateClips(afterSnap).map((c) => {
    const original = clips.find((f) => f.startMs === c.startMs && f.endMs === c.endMs)
    return (
      original ?? {
        title: c.title,
        startMs: c.startMs,
        endMs: c.endMs,
        topic: c.sourceTopic
      }
    )
  })
}

// --- Main pipeline ---

export async function analyzeTranscript(
  segments: TranscriptSegment[],
  provider: LLMProvider,
  model: string,
  apiKey: string,
  userHint?: string,
  onProgress?: (message: string, percent: number) => void,
  llmCall: LLMCaller = callLLM
): Promise<{ clips: ClipSegment[]; rawResponse: string }> {
  const formattedTranscript = formatTranscriptForLLM(segments)
  const totalDurationMs =
    segments.length > 0 ? segments[segments.length - 1].endMs - segments[0].startMs : 0
  const hint = userHint?.trim()
    ? `\n\nHard constraint from the user — discard anything that does not match: ${userHint.trim()}`
    : ''

  let debugOutput = ''
  const debugSink = (chunk: string): void => {
    debugOutput += chunk
  }

  // --- Phase 1: Hook finding ---
  onProgress?.('Finding peak moments...', 5)

  const hookResponse = await llmCall(
    provider,
    model,
    apiKey,
    HOOK_SYSTEM_PROMPT,
    `Here is the numbered transcript:\n\n${formattedTranscript}\n\nReturn the hottest hook lines.${hint}`
  )
  debugOutput += `=== PHASE 1: HOOKS ===\n${hookResponse}\n\n`

  const hooks = parseHooks(hookResponse, segments.length)
    .sort((a, b) => b.score - a.score)
    .slice(0, 15)

  onProgress?.('Finding peak moments...', 20)

  let allCandidates: CandidateClip[] = []

  if (hooks.length > 0) {
    const windows: MineWindow[] = hooks.map((hook) => {
      const hookSeg = segments[hook.hookId]
      const formatted = formatWindow(
        segments,
        hookSeg.startMs - 90000,
        hookSeg.endMs + 90000
      )
      return {
        label: `hook:${hook.hookId}`,
        userMessage:
          `Hook segment #${hook.hookId} (score ${hook.score}/10): "${hookSeg.text.trim()}"\nReason: ${hook.reason}\n\n${formatted}\n\nExpand this hook into a complete clip. Return a JSON array.${hint}`
      }
    })
    allCandidates = await mineWindows(
      windows,
      segments,
      provider,
      model,
      apiKey,
      llmCall,
      onProgress,
      debugSink
    )
  } else {
    debugOutput += `[No hooks found — falling back to time chunks]\n\n`
    const topics = fallbackToTimeChunks(segments)
    const windows: MineWindow[] = topics.map((topic) => {
      const formatted = formatWindow(
        segments,
        topic.startMs - 30000,
        topic.endMs + 30000,
        { startMs: topic.startMs, endMs: topic.endMs }
      )
      return {
        label: topic.topic,
        userMessage:
          `Section: ${topic.topic}. ${topic.description}\n\n${formatted}\n\nFind all clip-worthy moments in this section. Return a JSON array.${hint}`
      }
    })
    allCandidates = await mineWindows(
      windows,
      segments,
      provider,
      model,
      apiKey,
      llmCall,
      onProgress,
      debugSink
    )
  }

  // --- Global Rescue Fallback ---
  if (allCandidates.length === 0) {
    debugOutput += `=== GLOBAL RESCUE (no candidates) ===\n`
    const rescueResponse = await llmCall(
      provider,
      model,
      apiKey,
      CLIP_SYSTEM_PROMPT,
      `## NUMBERED TRANSCRIPT\n\n${formattedTranscript}\n\nSelect the best clips. Return only a JSON array.${hint}`
    )
    debugOutput += `${rescueResponse}\n\n`
    onProgress?.('Ranking and selecting best clips...', 90)
    const clips = parseLLMResponse(rescueResponse, segments).map((c) =>
      refineClipBounds(c, segments)
    )
    debugOutput += `=== FINAL OUTPUT (rescue) ===\n${JSON.stringify(clips, null, 2)}\n`
    return { clips, rawResponse: debugOutput }
  }

  allCandidates = budgetCandidates(allCandidates, 5)

  // --- Phase 3: Global Ranking ---
  onProgress?.('Ranking and selecting best clips...', 75)

  const candidateList = allCandidates
    .map((c, i) => {
      const full = getSegmentsInRange(segments, c.startMs, c.endMs)
        .map((s) => s.text.trim())
        .join(' ')
      return `${i + 1}. id=${i + 1} "${c.title}" [${msToTimecode(c.startMs)} - ${msToTimecode(c.endMs)}] Score: ${c.score}/10\n   Source: ${c.sourceTopic}\n   Why: ${c.justification}\n   Transcript: ${full}`
    })
    .join('\n\n')

  const rankingResponse = await llmCall(
    provider,
    model,
    apiKey,
    RANKING_SYSTEM_PROMPT,
    `Here are ${allCandidates.length} candidate clips:\n\n${candidateList}\n\nSelect the best clips by id. Return only a JSON array.`
  )
  debugOutput += `=== PHASE 3: GLOBAL RANKING ===\n${rankingResponse}\n\n`
  onProgress?.('Processing results...', 95)

  let finalClips: ClipSegment[] = []
  const ranked = parseRankedIds(rankingResponse)
  if (ranked.length > 0) {
    for (const row of ranked) {
      const cand = allCandidates[row.id - 1]
      if (!cand) continue
      finalClips.push({
        title: row.title?.trim() ? row.title : cand.title,
        startMs: cand.startMs,
        endMs: cand.endMs,
        category: row.category,
        topic: cand.sourceTopic
      })
    }
  } else {
    try {
      finalClips = parseLLMResponse(rankingResponse, segments)
    } catch {
      finalClips = allCandidates
        .sort((a, b) => b.score - a.score)
        .slice(0, 8)
        .map((c) => ({
          title: c.title,
          startMs: c.startMs,
          endMs: c.endMs,
          topic: c.sourceTopic
        }))
    }
  }

  if (finalClips.length === 0) {
    finalClips = allCandidates
      .sort((a, b) => b.score - a.score)
      .slice(0, 8)
      .map((c) => ({
        title: c.title,
        startMs: c.startMs,
        endMs: c.endMs,
        topic: c.sourceTopic
      }))
  }

  finalClips = assembleFinalClips(finalClips, allCandidates, segments, totalDurationMs)

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
