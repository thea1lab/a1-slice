import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs'
import { join, dirname, basename, resolve, sep } from 'path'
import { app } from 'electron'
import type {
  CaptionProject,
  ClipCrop,
  ClipSegment,
  CropRatio,
  ProjectData,
  TranscriptSegment,
  VideoInspection
} from '../shared/types'
import {
  formatTranscriptText,
  parseCaptions,
  parseClipsCache,
  parseFraming,
  rememberRecent,
  toStoredClip,
  transcriptTextPath
} from '../shared/project'

function a1Dir(): string {
  return join(app.getPath('home'), '.a1slice')
}

function ensureA1Dir(): void {
  if (!existsSync(a1Dir())) mkdirSync(a1Dir(), { recursive: true })
}

export function videoStem(videoPath: string): { dir: string; stem: string } {
  return {
    dir: dirname(videoPath),
    stem: basename(videoPath).replace(/\.[^.]+$/, '')
  }
}

function transcriptFile(videoPath: string): string {
  const { dir, stem } = videoStem(videoPath)
  return join(dir, `${stem}.a1slice.json`)
}

function clipsFile(videoPath: string): string {
  const { dir, stem } = videoStem(videoPath)
  return join(dir, `${stem}.a1slice-clips.json`)
}

function analysisFile(videoPath: string): string {
  const { dir, stem } = videoStem(videoPath)
  return join(dir, `${stem}.a1slice-analysis.txt`)
}

function framingFile(videoPath: string): string {
  const { dir, stem } = videoStem(videoPath)
  return join(dir, `${stem}.a1slice-framing.json`)
}

function captionsFile(videoPath: string): string {
  const { dir, stem } = videoStem(videoPath)
  return join(dir, `${stem}.a1slice-captions.json`)
}

function readJson(path: string): unknown {
  return JSON.parse(readFileSync(path, 'utf-8'))
}

export function readTranscript(videoPath: string): TranscriptSegment[] {
  try {
    if (!existsSync(transcriptFile(videoPath))) return []
    const parsed = readJson(transcriptFile(videoPath))
    if (!Array.isArray(parsed)) return []
    return parsed.filter(
      (segment): segment is TranscriptSegment =>
        !!segment &&
        typeof segment.startMs === 'number' &&
        typeof segment.endMs === 'number' &&
        typeof segment.text === 'string'
    )
  } catch {
    return []
  }
}

export function readClips(videoPath: string): { clips: ClipSegment[]; rawResponse: string } {
  const rawResponse = existsSync(analysisFile(videoPath))
    ? readFileSync(analysisFile(videoPath), 'utf-8')
    : ''
  try {
    if (existsSync(clipsFile(videoPath))) {
      return { clips: parseClipsCache(readJson(clipsFile(videoPath))), rawResponse }
    }
  } catch {
    return { clips: [], rawResponse }
  }
  return { clips: [], rawResponse }
}

export function readFraming(videoPath: string): Partial<Record<CropRatio, ClipCrop>> {
  try {
    if (!existsSync(framingFile(videoPath))) return {}
    return parseFraming(readJson(framingFile(videoPath)))
  } catch {
    return {}
  }
}

export function readCaptions(videoPath: string): CaptionProject | null {
  try {
    if (!existsSync(captionsFile(videoPath))) return null
    return parseCaptions(readJson(captionsFile(videoPath)))
  } catch {
    return null
  }
}

export function readProjectFiles(videoPath: string): Omit<ProjectData, 'durationMs'> {
  const { clips, rawResponse } = readClips(videoPath)
  return {
    segments: readTranscript(videoPath),
    clips,
    rawResponse,
    framing: readFraming(videoPath),
    captions: readCaptions(videoPath)
  }
}

export function inspectVideo(videoPath: string): VideoInspection {
  if (!videoPath || !existsSync(videoPath)) {
    return {
      hasTranscript: false,
      segmentCount: 0,
      hasClips: false,
      clipCount: 0,
      hasFraming: false,
      hasCaptions: false
    }
  }
  const segments = readTranscript(videoPath)
  const { clips } = readClips(videoPath)
  const framing = readFraming(videoPath)
  const captions = readCaptions(videoPath)
  return {
    hasTranscript: segments.length > 0,
    segmentCount: segments.length,
    hasClips: clips.length > 0,
    clipCount: clips.length,
    hasFraming: Object.keys(framing).length > 0,
    hasCaptions: captions !== null
  }
}

export function writeTranscript(videoPath: string, segments: TranscriptSegment[]): string {
  writeFileSync(transcriptFile(videoPath), JSON.stringify(segments), 'utf-8')
  const txtPath = transcriptTextPath(videoPath)
  const text = formatTranscriptText(segments)
  writeFileSync(txtPath, text ? `${text}\n` : '', 'utf-8')
  return txtPath
}

export function writeClips(videoPath: string, clips: ClipSegment[], rawResponse?: string): void {
  writeFileSync(clipsFile(videoPath), JSON.stringify(clips.map(toStoredClip)), 'utf-8')
  if (typeof rawResponse === 'string') {
    writeFileSync(analysisFile(videoPath), rawResponse, 'utf-8')
  }
}

export function writeFraming(
  videoPath: string,
  framing: Partial<Record<CropRatio, ClipCrop>>
): void {
  writeFileSync(framingFile(videoPath), JSON.stringify(framing), 'utf-8')
}

export function writeCaptions(videoPath: string, captions: CaptionProject): void {
  writeFileSync(captionsFile(videoPath), JSON.stringify(captions), 'utf-8')
}

function recentFile(): string {
  return join(a1Dir(), 'recent.json')
}

export function readRecent(): string[] {
  try {
    const parsed = readJson(recentFile())
    if (!Array.isArray(parsed)) return []
    return parsed.filter((item): item is string => typeof item === 'string')
  } catch {
    return []
  }
}

export function writeRemembered(videoPath: string): string[] {
  ensureA1Dir()
  const next = rememberRecent(readRecent(), videoPath)
  writeFileSync(recentFile(), JSON.stringify(next, null, 2), 'utf-8')
  return next
}

export function previewCacheDir(): string {
  return join(a1Dir(), 'previews')
}

export function cachedPreviewPath(key: string): string {
  return join(previewCacheDir(), `${key}.mp4`)
}

export function isCachedPreview(previewPath: string): boolean {
  const root = resolve(previewCacheDir())
  const file = resolve(previewPath)
  return file === root || file.startsWith(root + sep)
}
