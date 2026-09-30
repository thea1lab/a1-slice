import { formatTranscriptText } from './project'
import type { TranscriptSegment } from './types'

export interface CaptionEditRequest {
  fixTypos: boolean
  breakLines: boolean
  wordsPerLine: number
  note: string
}

export interface DiffRow {
  kind: 'same' | 'add' | 'remove' | 'gap'
  text: string
}

export function clampWordsPerLine(value: number): number {
  if (!Number.isFinite(value)) return 8
  return Math.min(24, Math.max(2, Math.round(value)))
}

export function formatCaptionClock(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000))
  const minutes = Math.floor(total / 60)
  const seconds = total % 60
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
}

export function captionLine(segment: TranscriptSegment): string {
  return `${formatCaptionClock(segment.startMs)}  ${segment.text.trim()}`
}

export function wordCount(segments: TranscriptSegment[]): number {
  return segments.reduce((total, segment) => {
    const words = segment.text.trim().split(/\s+/).filter(Boolean)
    return total + words.length
  }, 0)
}

export function coerceSegments(value: unknown): TranscriptSegment[] | null {
  if (!Array.isArray(value)) return null
  const segments: TranscriptSegment[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object') return null
    const row = item as Record<string, unknown>
    const startMs = typeof row.startMs === 'number' ? row.startMs : Number.NaN
    const endMs = typeof row.endMs === 'number' ? row.endMs : Number.NaN
    const text = typeof row.text === 'string' ? row.text.trim() : ''
    if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || startMs < 0) return null
    if (!text) continue
    const start = Math.round(startMs)
    const end = Math.round(endMs) > start ? Math.round(endMs) : start + 1
    segments.push({ startMs: start, endMs: end, text })
  }
  // Speech order wins. Sorting by time swaps a line that starts during the previous one.
  return segments.length > 0 ? segments : null
}

function repairSegmentTimes(segments: TranscriptSegment[]): TranscriptSegment[] {
  const next = segments.map((segment) => ({ ...segment }))
  for (let index = 0; index < next.length; index += 1) {
    if (index > 0 && next[index].startMs < next[index - 1].endMs) {
      if (next[index].startMs > next[index - 1].startMs) next[index - 1].endMs = next[index].startMs
      else next[index].startMs = next[index - 1].endMs
    }
    if (next[index].endMs <= next[index].startMs) next[index].endMs = next[index].startMs + 1
  }
  return next
}

export function guardCaptionEdit(before: TranscriptSegment[], after: TranscriptSegment[]): string | null {
  const original = wordCount(before)
  const next = wordCount(after)
  if (original === 0) return null
  if (next < original * 0.7) return 'The agent removed too many words. The lines were left as they are.'
  if (next > original * 1.3) return 'The agent added too many words. The lines were left as they are.'
  const last = before[before.length - 1]?.endMs ?? 0
  const drifted = after.some((segment) => segment.startMs > last + 5000)
  if (drifted) return 'The agent moved a line off the end of the video. The lines were left as they are.'
  return null
}

export function captionFileText(segments: TranscriptSegment[]): string {
  const body = formatTranscriptText(segments)
  return body ? `${body}\n` : ''
}

function evenChunks(words: string[], limit: number): string[][] {
  const chunks = Math.ceil(words.length / limit)
  const base = Math.floor(words.length / chunks)
  const extra = words.length % chunks
  const result: string[][] = []
  let cursor = 0
  for (let index = 0; index < chunks; index += 1) {
    const size = base + (index < extra ? 1 : 0)
    result.push(words.slice(cursor, cursor + size))
    cursor += size
  }
  return result
}

export function breakCaptionLines(segments: TranscriptSegment[], wordsPerLine: number): TranscriptSegment[] {
  const limit = clampWordsPerLine(wordsPerLine)
  const next: TranscriptSegment[] = []
  for (const segment of segments) {
    const parts = captionWords(segment.text)
    if (parts.length <= limit) {
      next.push(segment)
      continue
    }
    const chunks = evenChunks(parts, limit)
    const span = segment.endMs - segment.startMs
    let at = segment.startMs
    chunks.forEach((chunk, index) => {
      const end = index === chunks.length - 1 ? segment.endMs : at + Math.round((span * chunk.length) / parts.length)
      const endMs = Math.max(end, at + 1)
      next.push({ startMs: at, endMs, text: chunk.join(' ') })
      at = endMs
    })
  }
  return repairSegmentTimes(next)
}

export function isNoneAnswer(output: string): boolean {
  return /^none\.?$/i.test(output.trim())
}

export function captionEditPrompt(request: CaptionEditRequest, fileText: string): string {
  const words = clampWordsPerLine(request.wordsPerLine)
  const jobs: string[] = []
  if (request.fixTypos) jobs.push('- Fix typos and words that were clearly misheard.')
  if (request.breakLines) {
    jobs.push(`- Break any line longer than ${words} words into shorter lines. Keep every word, in the same order.`)
  }
  const note = request.note.trim()
  if (note) jobs.push(`- Also do this, and only this: ${note}`)
  if (jobs.length === 0) jobs.push('- Change nothing unless a line is obviously broken.')
  return [
    'Edit this transcript.',
    'Each line is one caption. The time stays at the start of the line.',
    '',
    'Do only this:',
    ...jobs,
    '',
    'Rules:',
    '- Keep the words in the same order.',
    '- Do not summarize, translate, or add facts.',
    '- When you split a line, leave the time on the first piece only.',
    '- Do not add blank lines.',
    request.breakLines
      ? '- Print only the edited transcript. Do not describe what you are doing.'
      : '- Do not join or split lines. If nothing needs a change, print exactly NONE. Otherwise print only the edited transcript. Do not describe what you are doing.',
    '',
    fileText.trim()
  ].join('\n')
}

function stripStamp(line: string): string {
  const clocked = line.match(/^(?:\d+:)?\d{1,2}:\d{2}\s+(.*)$/)
  return (clocked ? clocked[1] : line).trim()
}

export function captionFileLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map(stripStamp)
    .filter((line) => line.length > 0)
}

function captionWords(text: string): string[] {
  return text.trim().split(/\s+/).filter(Boolean)
}

function sameWord(left: string, right: string): boolean {
  const clean = (word: string): string => word.toLowerCase().replace(/[^\p{L}\p{N}']/gu, '')
  const a = clean(left)
  const b = clean(right)
  return a.length > 0 && a === b
}

interface TimedWord {
  word: string
  startMs: number
  endMs: number
  segmentIndex: number
}

function wordTimeline(segments: TranscriptSegment[]): TimedWord[] {
  const timeline: TimedWord[] = []
  segments.forEach((segment, segmentIndex) => {
    const parts = captionWords(segment.text)
    const span = segment.endMs - segment.startMs
    parts.forEach((word, index) => {
      const startMs = Math.round(segment.startMs + (span * index) / parts.length)
      const endMs = Math.round(segment.startMs + (span * (index + 1)) / parts.length)
      timeline.push({ word, startMs, endMs: Math.max(endMs, startMs + 1), segmentIndex })
    })
  })
  return timeline
}

function alignWords(original: string[], edited: string[]): number[] | null {
  const map: number[] = []
  let i = 0
  let j = 0
  let chaos = 0
  while (j < edited.length) {
    if (i >= original.length) {
      map.push(Math.max(0, original.length - 1))
      chaos += 1
      j += 1
      continue
    }
    if (sameWord(original[i], edited[j])) {
      map.push(i)
      i += 1
      j += 1
      continue
    }
    let ahead = -1
    for (let step = 1; step <= 3 && i + step < original.length; step += 1) {
      if (sameWord(original[i + step], edited[j])) {
        ahead = i + step
        break
      }
    }
    if (ahead >= 0) {
      chaos += ahead - i
      i = ahead
      map.push(i)
      i += 1
      j += 1
      continue
    }
    let inserted = -1
    for (let step = 1; step <= 3 && j + step < edited.length; step += 1) {
      if (sameWord(original[i], edited[j + step])) {
        inserted = step
        break
      }
    }
    if (inserted !== null && inserted >= 0) {
      for (let step = 0; step < inserted; step += 1) {
        map.push(i)
        chaos += 1
        j += 1
      }
      continue
    }
    map.push(i)
    i += 1
    j += 1
  }
  chaos += original.length - i
  if (original.length > 0 && chaos > original.length * 0.35) return null
  return map
}

export function applyEditedCaptionText(
  before: TranscriptSegment[],
  editedText: string
): { segments: TranscriptSegment[] } | { error: string } {
  const lines = captionFileLines(editedText)
  if (lines.length === 0) {
    return { error: 'The agent did not return the edited lines. The lines were left as they are.' }
  }
  const timeline = wordTimeline(before)
  if (timeline.length === 0) {
    return { error: 'The agent returned lines that could not be read. The lines were left as they are.' }
  }
  const editedWords = lines.flatMap(captionWords)
  const aligned = alignWords(
    timeline.map((item) => item.word),
    editedWords
  )
  if (!aligned || aligned.length !== editedWords.length) {
    return { error: 'The agent changed too much of the file. The lines were left as they are.' }
  }

  const built: TranscriptSegment[] = []
  const owners: number[] = []
  let cursor = 0
  for (const line of lines) {
    const count = captionWords(line).length
    const indices = aligned.slice(cursor, cursor + count)
    cursor += count
    const ownersInLine = new Set(indices.map((index) => timeline[index].segmentIndex))
    owners.push(ownersInLine.size === 1 ? [...ownersInLine][0] : -1)
    const first = timeline[indices[0]]
    const last = timeline[indices[indices.length - 1]]
    built.push({ startMs: first.startMs, endMs: Math.max(last.endMs, first.startMs + 1), text: line })
  }

  let lineIndex = 0
  while (lineIndex < built.length) {
    const owner = owners[lineIndex]
    if (owner < 0) {
      lineIndex += 1
      continue
    }
    let end = lineIndex + 1
    while (end < built.length && owners[end] === owner) end += 1
    const segment = before[owner]
    const group = built.slice(lineIndex, end)
    const counts = group.map((item) => captionWords(item.text).length)
    const total = counts.reduce((sum, count) => sum + count, 0)
    const span = segment.endMs - segment.startMs
    let at = segment.startMs
    group.forEach((item, index) => {
      const next = index === group.length - 1 ? segment.endMs : at + Math.round((span * counts[index]) / total)
      item.startMs = at
      item.endMs = Math.max(next, at + 1)
      at = item.endMs
    })
    lineIndex = end
  }

  const repaired = repairSegmentTimes(built)
  built.splice(0, built.length, ...repaired)

  const guard = guardCaptionEdit(before, built)
  if (guard) return { error: guard }
  return { segments: built }
}

const captionStamp = /^(?:\d+:)?\d{1,2}:\d{2}\s+\S/

function stripAnsi(text: string): string {
  return text.replace(/\u001b\[[0-9;]*m/g, '')
}

function captionBlocks(text: string): string[] {
  const blocks: string[] = []
  let current: string[] | null = null
  for (const line of stripAnsi(text).split(/\r?\n/)) {
    if (line.trim() === '') {
      if (current && current.length > 0) blocks.push(current.join('\n').trim())
      current = null
      continue
    }
    if (current) {
      current.push(line)
      continue
    }
    if (captionStamp.test(line.trim())) current = [line]
  }
  if (current && current.length > 0) blocks.push(current.join('\n').trim())
  return blocks.filter((block) => captionFileLines(block).length > 0)
}

export function extractPrintedCaption(output: string): string | null {
  const fenced = output.match(/```(?:text|txt)?\s*([\s\S]*?)```/)
  const blocks = captionBlocks(fenced && captionBlocks(fenced[1]).length > 0 ? fenced[1] : output)
  return blocks.at(-1) ?? null
}

export function captionLogLine(line: string, state: { hidingFile: boolean }): string | null {
  const trimmed = stripAnsi(line).trim()
  if (!trimmed) return null
  if (captionStamp.test(trimmed) || state.hidingFile) {
    const started = state.hidingFile
    state.hidingFile = true
    return started ? null : 'Writing the edited transcript.'
  }
  return trimmed
}

export function parseAgentCaptionEdit(output: string): unknown {
  const candidates: string[] = []
  const start = output.indexOf('CAPTIONS_JSON_START')
  const end = output.indexOf('CAPTIONS_JSON_END')
  if (start !== -1 && end > start) {
    candidates.push(output.slice(start + 'CAPTIONS_JSON_START'.length, end).trim())
  }
  const fenced = output.match(/```(?:json)?\s*([\s\S]*?)```/)
  if (fenced) candidates.push(fenced[1].trim())
  const first = output.indexOf('[')
  const last = output.lastIndexOf(']')
  if (first !== -1 && last > first) candidates.push(output.slice(first, last + 1))
  for (const candidate of candidates) {
    try {
      return JSON.parse(candidate)
    } catch {
      continue
    }
  }
  throw new Error('no json')
}

export function acceptAgentEdit(
  before: TranscriptSegment[],
  output: string
): { segments: TranscriptSegment[] } | { error: string } {
  let parsed: unknown
  try {
    parsed = parseAgentCaptionEdit(output)
  } catch {
    return { error: 'The agent did not return the edited lines. The lines were left as they are.' }
  }
  const segments = coerceSegments(parsed)
  if (!segments) {
    return { error: 'The agent returned lines that could not be read. The lines were left as they are.' }
  }
  const guard = guardCaptionEdit(before, segments)
  if (guard) return { error: guard }
  return { segments }
}

export function diffCaptionLines(before: string[], after: string[]): DiffRow[] {
  const n = before.length
  const m = after.length
  const scores: number[][] = Array.from({ length: n + 1 }, () => Array<number>(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      scores[i][j] =
        before[i] === after[j] ? scores[i + 1][j + 1] + 1 : Math.max(scores[i + 1][j], scores[i][j + 1])
    }
  }
  const rows: DiffRow[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (before[i] === after[j]) {
      rows.push({ kind: 'same', text: before[i] })
      i += 1
      j += 1
    } else if (scores[i + 1][j] >= scores[i][j + 1]) {
      rows.push({ kind: 'remove', text: before[i] })
      i += 1
    } else {
      rows.push({ kind: 'add', text: after[j] })
      j += 1
    }
  }
  while (i < n) rows.push({ kind: 'remove', text: before[i++] })
  while (j < m) rows.push({ kind: 'add', text: after[j++] })
  return rows
}

export function compactDiff(rows: DiffRow[], context = 2): DiffRow[] {
  const keep = new Set<number>()
  rows.forEach((row, index) => {
    if (row.kind === 'same') return
    for (let cursor = Math.max(0, index - context); cursor <= Math.min(rows.length - 1, index + context); cursor++) {
      keep.add(cursor)
    }
  })
  const compact: DiffRow[] = []
  let skipped = false
  rows.forEach((row, index) => {
    if (!keep.has(index)) {
      skipped = true
      return
    }
    if (skipped) {
      compact.push({ kind: 'gap', text: '…' })
      skipped = false
    }
    compact.push(row)
  })
  return compact
}

export function diffSummary(rows: DiffRow[]): string {
  const removed = rows.filter((row) => row.kind === 'remove').length
  const added = rows.filter((row) => row.kind === 'add').length
  if (removed === 0 && added === 0) return 'The agent left the words as they were.'
  const parts: string[] = []
  if (removed > 0) parts.push(removed === 1 ? 'Removed 1 line.' : `Removed ${removed} lines.`)
  if (added > 0) parts.push(added === 1 ? 'Added 1 line.' : `Added ${added} lines.`)
  return parts.join(' ')
}

export interface AgentLogState {
  hideJson: boolean
  finalText: string | null
  pieces: string[]
}

export function emptyAgentLogState(): AgentLogState {
  return { hideJson: false, finalText: null, pieces: [] }
}

export function visibleLogLine(line: string, state: { hideJson: boolean }): string | null {
  const trimmed = line.replace(/\u001b\[[0-9;]*m/g, '').trim()
  if (!trimmed) return null
  if (trimmed.includes('CAPTIONS_JSON_START')) {
    state.hideJson = true
    return 'Writing the edited captions.'
  }
  if (trimmed.includes('CAPTIONS_JSON_END')) {
    state.hideJson = false
    return null
  }
  if (state.hideJson || trimmed.startsWith('```')) return null
  if (trimmed.startsWith('NOTE:')) return trimmed.slice('NOTE:'.length).trim()
  return trimmed
}

export function interpretAgentLine(line: string, state: AgentLogState, streamJson: boolean): string[] {
  if (!streamJson) {
    state.pieces.push(line)
    const shown = visibleLogLine(line, state)
    return shown ? [shown] : []
  }
  let event: Record<string, unknown>
  try {
    event = JSON.parse(line) as Record<string, unknown>
  } catch {
    const shown = visibleLogLine(line, state)
    return shown ? [shown] : []
  }
  if (event.type === 'result' && typeof event.result === 'string') {
    state.finalText = event.result
    if (state.pieces.length > 0) return []
    const logs: string[] = []
    for (const part of event.result.split(/\r?\n/)) {
      const shown = visibleLogLine(part, state)
      if (shown) logs.push(shown)
    }
    return logs
  }
  if (event.type !== 'assistant' || !event.message || typeof event.message !== 'object') return []
  const content = (event.message as { content?: unknown }).content
  if (!Array.isArray(content)) return []
  const logs: string[] = []
  for (const block of content) {
    if (!block || typeof block !== 'object') continue
    const row = block as Record<string, unknown>
    if (row.type === 'tool_use') {
      logs.push(row.name === 'Read' ? 'Reading the captions.' : `Using ${String(row.name)}.`)
    }
    if (row.type === 'text' && typeof row.text === 'string') {
      state.pieces.push(row.text)
      for (const part of row.text.split(/\r?\n/)) {
        const shown = visibleLogLine(part, state)
        if (shown) logs.push(shown)
      }
    }
  }
  return logs
}

export function agentOutputText(state: AgentLogState): string {
  if (state.finalText !== null) return state.finalText
  return state.pieces.join('\n')
}
