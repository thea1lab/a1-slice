import ffmpegPath from 'ffmpeg-static'
import { spawn } from 'child_process'
import { tmpdir } from 'os'
import { join } from 'path'
import { randomUUID } from 'crypto'
import { writeFileSync, unlinkSync } from 'fs'
import type { TranscriptSegment } from '../shared/types'

const FFMPEG = ffmpegPath as string

export function formatSrtTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  const millis = ms % 1000
  return (
    String(hours).padStart(2, '0') +
    ':' +
    String(minutes).padStart(2, '0') +
    ':' +
    String(seconds).padStart(2, '0') +
    ',' +
    String(millis).padStart(3, '0')
  )
}

export function generateSrt(segments: TranscriptSegment[]): string {
  return segments
    .map((seg, i) => {
      const idx = i + 1
      const start = formatSrtTime(seg.startMs)
      const end = formatSrtTime(seg.endMs)
      return `${idx}\n${start} --> ${end}\n${seg.text.trim()}\n`
    })
    .join('\n')
}

function parseProgress(stderr: string): number | null {
  // ffmpeg outputs "time=HH:MM:SS.mm" in progress lines
  const match = stderr.match(/time=(\d+):(\d+):(\d+)\.(\d+)/)
  if (!match) return null
  const h = parseInt(match[1], 10)
  const m = parseInt(match[2], 10)
  const s = parseInt(match[3], 10)
  return h * 3600 + m * 60 + s
}

function spawnFfmpeg(
  args: string[],
  durationSec: number | null,
  onProgress?: (percent: number) => void
): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(FFMPEG, args)
    let stderr = ''

    proc.stderr.on('data', (chunk: Buffer) => {
      stderr += chunk.toString()
      if (onProgress && durationSec && durationSec > 0) {
        const currentSec = parseProgress(stderr)
        if (currentSec != null) {
          onProgress(Math.min(100, Math.round((currentSec / durationSec) * 100)))
        }
      }
    })

    proc.on('close', (code) => {
      if (code === 0) resolve()
      else reject(new Error(`ffmpeg exited with code ${code}:\n${stderr.slice(-500)}`))
    })

    proc.on('error', (err) => reject(err))
  })
}

export async function extractAudio(
  videoPath: string,
  onProgress?: (percent: number) => void
): Promise<string> {
  const outPath = join(tmpdir(), `a1slice-${randomUUID()}.wav`)
  await spawnFfmpeg(
    ['-i', videoPath, '-ar', '16000', '-ac', '1', '-f', 'wav', '-y', outPath],
    null,
    onProgress
  )
  return outPath
}

export async function cutClipWithSubtitles(
  videoPath: string,
  outputPath: string,
  startMs: number,
  endMs: number,
  subtitles: TranscriptSegment[],
  onProgress?: (percent: number) => void
): Promise<string> {
  // Shift subtitle times relative to clip start
  const shifted = subtitles
    .filter((s) => s.endMs > startMs && s.startMs < endMs)
    .map((s) => ({
      startMs: Math.max(0, s.startMs - startMs),
      endMs: Math.min(endMs - startMs, s.endMs - startMs),
      text: s.text
    }))

  const srtPath = join(tmpdir(), `a1slice-sub-${randomUUID()}.srt`)
  writeFileSync(srtPath, generateSrt(shifted), 'utf-8')

  const startSec = startMs / 1000
  const durationSec = (endMs - startMs) / 1000

  // Escape the srt path for the subtitles filter
  const escapedSrt = srtPath.replace(/\\/g, '\\\\').replace(/:/g, '\\:').replace(/'/g, "'\\''")
  const subFilter =
    `subtitles='${escapedSrt}'` +
    `:force_style='FontSize=24,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,Outline=2'`

  try {
    await spawnFfmpeg(
      [
        '-ss', String(startSec),
        '-i', videoPath,
        '-t', String(durationSec),
        '-vf', subFilter,
        '-c:a', 'aac', '-b:a', '128k',
        '-y', outputPath
      ],
      durationSec,
      onProgress
    )
  } finally {
    try { unlinkSync(srtPath) } catch {}
  }

  return outputPath
}
