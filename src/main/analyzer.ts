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

const ANALYSIS_SYSTEM_PROMPT = `You are a video content analyst. You will receive a timestamped transcript of a video. Your job is to thoroughly analyze the video content to prepare for clip selection.

Produce a structured analysis with the following sections:

## SUMMARY
What is this video about? Who is speaking? What is the overall value/quality of the content?

## MAIN TOPIC
One sentence describing the core subject.

## SECTION BREAKDOWN
Break the video into logical sections. For each section:
- Time range (use the timecodes from the transcript)
- Brief description of content
- Value rating: HIGH / MEDIUM / LOW
- Standalone potential: YES / NO (could this section make sense without the rest of the video?)

## KEY MOMENTS
List specific timestamps where something notable happens — a great insight, a compelling statement, an emotional moment, a demonstration payoff, a surprising reveal, humor, etc.

## DEAD ZONES
List any sections that should be avoided for clips — low-energy filler, repetitive content, off-topic tangents, poor audio/speaking quality, or segments that only make sense with extensive context.

Be thorough. Cover the ENTIRE video. Your analysis will be used to select the best clips, so missing a great moment means it won't become a clip.`

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

export async function analyzeTranscript(
  segments: TranscriptSegment[],
  provider: LLMProvider,
  model: string,
  apiKey: string,
  userHint?: string,
  onProgress?: (message: string, percent: number) => void
): Promise<{ clips: ClipSegment[]; rawResponse: string }> {
  const formattedTranscript = formatTranscriptForLLM(segments)

  // Phase 1: Video analysis
  onProgress?.('Understanding video content...', 10)

  let analysisUserMessage = `Here is the transcript:\n\n${formattedTranscript}\n\nAnalyze this video thoroughly.`
  if (userHint?.trim()) {
    analysisUserMessage += `\n\nAdditional context from the user: ${userHint.trim()}`
  }

  const videoAnalysis = await callLLM(
    provider, model, apiKey,
    ANALYSIS_SYSTEM_PROMPT, analysisUserMessage
  )

  // Phase 2: Clip selection
  onProgress?.('Selecting best clips...', 55)

  let clipUserMessage = `## VIDEO ANALYSIS\n\n${videoAnalysis}\n\n## ORIGINAL TRANSCRIPT\n\n${formattedTranscript}\n\nBased on the analysis above, select the best clips. Return only a JSON array.`
  if (userHint?.trim()) {
    clipUserMessage += `\n\nUser preferences: ${userHint.trim()}`
  }

  const clipResponse = await callLLM(
    provider, model, apiKey,
    CLIP_SYSTEM_PROMPT, clipUserMessage
  )

  onProgress?.('Processing results...', 95)

  return { clips: parseLLMResponse(clipResponse), rawResponse: clipResponse }
}
