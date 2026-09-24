import type {
  CaptionProject,
  ClipCrop,
  ClipSegment,
  ClipSegmentWithStatus,
  CropRatio,
  TranscriptSegment
} from './types'
import { DEFAULT_CROP } from './types'

const CROP_RATIOS = new Set<CropRatio>(['original', '16:9', '4:3', '9:16', '1:1'])

export function parseCrop(value: unknown): ClipCrop | null {
  if (!value || typeof value !== 'object') return null
  const item = value as Record<string, unknown>
  if (typeof item.ratio !== 'string' || !CROP_RATIOS.has(item.ratio as CropRatio)) return null
  if (typeof item.cx !== 'number' || typeof item.cy !== 'number' || typeof item.zoom !== 'number') {
    return null
  }
  if (![item.cx, item.cy, item.zoom].every(Number.isFinite)) return null
  const ratio = item.ratio as CropRatio
  return {
    ratio,
    cx: Math.min(1, Math.max(0, item.cx)),
    cy: Math.min(1, Math.max(0, item.cy)),
    zoom: Math.min(1, Math.max(0, item.zoom))
  }
}

function parseSegments(value: unknown): TranscriptSegment[] {
  if (!Array.isArray(value)) return []
  const segments: TranscriptSegment[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object') continue
    const row = item as Record<string, unknown>
    if (typeof row.startMs !== 'number' || typeof row.endMs !== 'number' || typeof row.text !== 'string') {
      continue
    }
    if (!Number.isFinite(row.startMs) || !Number.isFinite(row.endMs) || row.endMs <= row.startMs) continue
    segments.push({ startMs: row.startMs, endMs: row.endMs, text: row.text })
  }
  return segments
}

export function parseClipsCache(value: unknown): ClipSegment[] {
  if (!Array.isArray(value)) return []
  const clips: ClipSegment[] = []
  for (const item of value) {
    if (!item || typeof item !== 'object') continue
    const row = item as Record<string, unknown>
    if (typeof row.title !== 'string' || typeof row.startMs !== 'number' || typeof row.endMs !== 'number') {
      continue
    }
    if (!Number.isFinite(row.startMs) || !Number.isFinite(row.endMs)) continue
    const clip: ClipSegment = {
      title: row.title,
      startMs: row.startMs,
      endMs: row.endMs
    }
    if (row.category === 'related' || row.category === 'standalone') clip.category = row.category
    if (typeof row.topic === 'string' && row.topic) clip.topic = row.topic
    if (row.approved === false) clip.approved = false
    else if (row.approved === true) clip.approved = true
    const crop = parseCrop(row.crop)
    if (crop) clip.crop = crop
    clips.push(clip)
  }
  return clips
}

export function toStoredClip(clip: ClipSegment & { id?: string }): ClipSegment {
  const stored: ClipSegment = {
    title: clip.title,
    startMs: clip.startMs,
    endMs: clip.endMs,
    approved: clip.approved !== false
  }
  if (clip.category) stored.category = clip.category
  if (clip.topic) stored.topic = clip.topic
  if (clip.crop) stored.crop = clip.crop
  return stored
}

export function withClipStatus(clips: ClipSegment[]): ClipSegmentWithStatus[] {
  return clips.map((clip, index) => ({
    ...clip,
    id: String(index),
    approved: clip.approved !== false,
    crop: clip.crop ? { ...clip.crop } : { ...DEFAULT_CROP }
  }))
}

export function parseFraming(value: unknown): Partial<Record<CropRatio, ClipCrop>> {
  if (!value || typeof value !== 'object') return {}
  const framing: Partial<Record<CropRatio, ClipCrop>> = {}
  for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
    if (!CROP_RATIOS.has(key as CropRatio)) continue
    const crop = parseCrop(raw)
    if (!crop || crop.ratio !== key) continue
    framing[key as CropRatio] = crop
  }
  return framing
}

export function parseCaptions(value: unknown): CaptionProject | null {
  if (!value || typeof value !== 'object') return null
  const row = value as Record<string, unknown>
  const source = row.source === 'manual' || row.source === 'transcript' ? row.source : null
  const look =
    row.look === 'srt' || row.look === 'burn-large' || row.look === 'burn-small' ? row.look : null
  if (!source || !look) return null
  return { source, look, cues: parseSegments(row.cues) }
}

export function rememberRecent(paths: string[], videoPath: string, max = 8): string[] {
  if (!videoPath) return paths.slice(0, max)
  return [videoPath, ...paths.filter((path) => path !== videoPath)].slice(0, max)
}

export function transcriptTextPath(videoPath: string): string {
  const base = videoPath.split(/[\\/]/).pop() || videoPath
  const stem = base.replace(/\.[^.]+$/, '')
  const name = `${stem}-transcript.txt`
  const slash = Math.max(videoPath.lastIndexOf('/'), videoPath.lastIndexOf('\\'))
  if (slash < 0) return name
  return `${videoPath.slice(0, slash + 1)}${name}`
}

export function formatTranscriptText(segments: TranscriptSegment[]): string {
  return segments
    .map((segment) => {
      const total = Math.max(0, Math.floor(segment.startMs / 1000))
      const hours = Math.floor(total / 3600)
      const minutes = Math.floor((total % 3600) / 60)
      const seconds = total % 60
      const stamp =
        hours > 0
          ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
          : `${minutes}:${String(seconds).padStart(2, '0')}`
      return `${stamp}  ${segment.text.trim()}`
    })
    .join('\n')
}

function srtClockToMs(hours: string, minutes: string, seconds: string, fraction: string): number {
  const millis = fraction.length >= 3
    ? parseInt(fraction.slice(0, 3), 10)
    : Math.round(parseInt(fraction, 10) * 10 ** (3 - fraction.length))
  return (
    parseInt(hours, 10) * 3600000 +
    parseInt(minutes, 10) * 60000 +
    parseInt(seconds, 10) * 1000 +
    millis
  )
}

export function parseSrt(text: string): TranscriptSegment[] {
  const blocks = text.replace(/^\uFEFF/, '').trim().split(/\r?\n\s*\r?\n/)
  const segments: TranscriptSegment[] = []
  for (const block of blocks) {
    const lines = block.split(/\r?\n/).map((line) => line.trim()).filter(Boolean)
    const timeIndex = lines.findIndex((line) => line.includes('-->'))
    if (timeIndex < 0) continue
    const match = lines[timeIndex].match(
      /(\d+):(\d{2}):(\d{2})[,.](\d{1,3})\s*-->\s*(\d+):(\d{2}):(\d{2})[,.](\d{1,3})/
    )
    if (!match) continue
    const startMs = srtClockToMs(match[1], match[2], match[3], match[4])
    const endMs = srtClockToMs(match[5], match[6], match[7], match[8])
    const cue = lines.slice(timeIndex + 1).join('\n').trim()
    if (!cue || endMs <= startMs) continue
    segments.push({ startMs, endMs, text: cue })
  }
  return segments
}

export function previewCacheKey(
  videoPath: string,
  size: number,
  mtimeMs: number,
  fileStartMs: number,
  fileEndMs: number
): string {
  const raw = `${videoPath}\0${size}\0${Math.round(mtimeMs)}\0${Math.round(fileStartMs)}\0${Math.round(fileEndMs)}`
  let hash = 0x811c9dc5
  for (let i = 0; i < raw.length; i++) {
    hash ^= raw.charCodeAt(i)
    hash = Math.imul(hash, 0x01000193)
  }
  return (hash >>> 0).toString(16).padStart(8, '0')
}
