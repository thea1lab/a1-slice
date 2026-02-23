import Anthropic from '@anthropic-ai/sdk'
import OpenAI from 'openai'
import type {
  TranscriptSegment,
  ClipSegment,
  LLMProvider
} from '../shared/types'

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
        `[${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)}] ${seg.text.trim()}`
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
    (item: { title: string; start_ms: number; end_ms: number }) => ({
      title: item.title,
      startMs: item.start_ms,
      endMs: item.end_ms
    })
  )
}

const SYSTEM_PROMPT = `You are a video editor AI. You will receive a timestamped transcript of a long video. Your job is to identify the best self-contained segments that would work as short-form clips (30 seconds to 3 minutes each).

Pick segments that:
- Tell a complete story or make a complete point
- Are engaging, funny, insightful, or emotionally compelling
- Have clean start and end points (not mid-sentence)
- Would work as standalone clips without additional context

Return a JSON array (no other text) where each element has:
- "title": a short, catchy title for the clip
- "start_ms": start time in milliseconds
- "end_ms": end time in milliseconds

Example:
[
  {"title": "The moment everything changed", "start_ms": 45000, "end_ms": 120000},
  {"title": "Best advice for beginners", "start_ms": 300000, "end_ms": 420000}
]`

export async function analyzeTranscript(
  segments: TranscriptSegment[],
  provider: LLMProvider,
  model: string,
  apiKey: string
): Promise<ClipSegment[]> {
  const formattedTranscript = formatTranscriptForLLM(segments)
  const userMessage = `Here is the transcript:\n\n${formattedTranscript}\n\nIdentify the best clips from this transcript. Return only a JSON array.`

  let responseText: string

  if (provider === 'claude') {
    const client = new Anthropic({ apiKey })
    const response = await client.messages.create({
      model,
      max_tokens: 4096,
      system: SYSTEM_PROMPT,
      messages: [{ role: 'user', content: userMessage }]
    })
    const block = response.content[0]
    responseText = block.type === 'text' ? block.text : ''
  } else {
    const client = new OpenAI({ apiKey })
    const response = await client.chat.completions.create({
      model,
      max_tokens: 4096,
      messages: [
        { role: 'system', content: SYSTEM_PROMPT },
        { role: 'user', content: userMessage }
      ]
    })
    responseText = response.choices[0]?.message?.content ?? ''
  }

  return parseLLMResponse(responseText)
}
