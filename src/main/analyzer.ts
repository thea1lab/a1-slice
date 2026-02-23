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
        `[${seg.startMs}ms -> ${seg.endMs}ms] ${seg.text.trim()}`
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

const SYSTEM_PROMPT = `You are a video editor AI. You will receive a timestamped transcript of a video. Your job is to pick ONLY the most impactful, high-value moments — the parts that hit hardest and would perform well as standalone short-form clips.

CRITICAL RULES:
- The transcript uses millisecond timestamps (e.g. "13000ms -> 30000ms"). Your start_ms and end_ms values MUST use these exact millisecond values from the transcript. Do NOT convert or calculate — just copy the numbers directly.
- Be selective. Not every part of the video deserves a clip. It's better to return 1-2 great clips than 4 mediocre ones.
- Clips should be 20 seconds to 2 minutes each.
- You do NOT need to cover the entire video. Skip boring, repetitive, or low-energy sections.

Pick moments that:
- Have a clear "wow" factor — a key insight, demo payoff, or compelling statement
- Tell a complete story or make a complete point
- Have clean start and end points (not mid-sentence)
- Would work as standalone clips without additional context

Return a JSON array (no other text) where each element has:
- "title": a short, catchy title for the clip
- "start_ms": start time in milliseconds (must be a timestamp that appears in the transcript)
- "end_ms": end time in milliseconds (must be a timestamp that appears in the transcript)

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
): Promise<{ clips: ClipSegment[]; rawResponse: string }> {
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

  return { clips: parseLLMResponse(responseText), rawResponse: responseText }
}
