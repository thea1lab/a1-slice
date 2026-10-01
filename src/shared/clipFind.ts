import type { TranscriptSegment } from './types'

function msToTimecode(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const h = Math.floor(totalSeconds / 3600)
  const m = Math.floor((totalSeconds % 3600) / 60)
  const s = totalSeconds % 60
  return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
}

export function formatClipTranscript(segments: TranscriptSegment[]): string {
  return segments
    .map(
      (seg, i) =>
        `[#${i} ${msToTimecode(seg.startMs)} -> ${msToTimecode(seg.endMs)} | ${seg.startMs} -> ${seg.endMs}] ${seg.text.trim()}`
    )
    .join('\n')
}

export function clipFindPrompt(segments: TranscriptSegment[], userHint?: string): string {
  const hint = userHint?.trim() ?? ''
  const constraint = hint
    ? `\n\nHard constraint from the user — discard anything that does not match: ${hint}`
    : ''
  return `You are a video editor. You will receive a numbered transcript. Each line starts with [#N ...]. N is the segment id.

Select the best clips from this video.

RULES:
- Target duration 20-90 seconds. About 45-60 seconds is ideal.
- Each clip must be a complete thought with a hook and a payoff.
- Use start_id and end_id from the #N ids. Do not invent milliseconds. Do not convert timecodes.
- Skip greetings, filler, and moments that only introduce the class.
- Return 3 to 8 clips. Prefer the strongest moments. Do not cover the whole video.
- Write each title in the language of the transcript.
- Clips must not overlap.

CATEGORIES:
- "related": about the lecture's main topic
- "standalone": understandable with no extra context

Return a JSON array and no other text:
[{"title":"short title","start_id":10,"end_id":18,"category":"standalone"}]

If nothing is worth clipping, return [].

## NUMBERED TRANSCRIPT

${formatClipTranscript(segments)}
${constraint}`
}
