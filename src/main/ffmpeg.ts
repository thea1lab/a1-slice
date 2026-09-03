import ffmpegPath from 'ffmpeg-static'
import { spawn } from 'child_process'
import { tmpdir } from 'os'
import { join } from 'path'
import { randomUUID } from 'crypto'
import { writeFileSync } from 'fs'
import type { ClipCrop, TranscriptSegment } from '../shared/types'
import { ffmpegCropFilter } from '../shared/crop'

const FFMPEG = (ffmpegPath as string).replace('app.asar', 'app.asar.unpacked')

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

export function parseFfmpegProgress(stderr: string): number | null {
  let last: RegExpExecArray | null = null
  const re = /time=(\d+):(\d+):(\d+)\.(\d+)/g
  for (let match = re.exec(stderr); match; match = re.exec(stderr)) {
    last = match
  }
  if (!last) return null
  const h = parseInt(last[1], 10)
  const m = parseInt(last[2], 10)
  const s = parseInt(last[3], 10)
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
        const currentSec = parseFfmpegProgress(stderr)
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
    ['-i', videoPath, '-af', 'afftdn=nr=12:nf=-20:tn=1', '-ar', '16000', '-ac', '1', '-f', 'wav', '-y', outPath],
    null,
    onProgress
  )
  return outPath
}

export async function splitWav(
  wavPath: string,
  chunkSec = 300,
  overlapSec = 15
): Promise<{ path: string; offsetMs: number; durationMs: number }[]> {
  const durationMs = await getVideoDurationMs(wavPath)
  const overlap = overlapSec >= chunkSec ? 0 : Math.max(0, overlapSec)
  const hopSec = Math.max(1, chunkSec - overlap)
  const chunks: { path: string; offsetMs: number; durationMs: number }[] = []
  for (let offsetSec = 0; offsetSec * 1000 < durationMs; offsetSec += hopSec) {
    const chunkPath = wavPath.replace('.wav', `-chunk${chunks.length}.wav`)
    await spawnFfmpeg(
      [
        '-ss', String(offsetSec),
        '-i', wavPath,
        '-t', String(chunkSec),
        '-ar', '16000',
        '-ac', '1',
        '-y',
        chunkPath
      ],
      null
    )
    let chunkDurationMs = Math.min(chunkSec * 1000, Math.max(0, durationMs - offsetSec * 1000))
    try {
      chunkDurationMs = await getVideoDurationMs(chunkPath)
    } catch {
      // keep the nominal duration if ffprobe-style parse fails
    }
    chunks.push({ path: chunkPath, offsetMs: offsetSec * 1000, durationMs: chunkDurationMs })
  }
  return chunks
}

export function parseFfmpegDuration(stderr: string): number | null {
  const match = stderr.match(/Duration:\s*(\d+):(\d+):(\d+)\.(\d+)/)
  if (!match) return null
  const hours = parseInt(match[1], 10)
  const minutes = parseInt(match[2], 10)
  const seconds = parseInt(match[3], 10)
  const ms = Math.round(parseFloat(`0.${match[4]}`) * 1000)
  return hours * 3600000 + minutes * 60000 + seconds * 1000 + ms
}

export function parseFfmpegVideoSize(
  stderr: string
): { width: number; height: number } | null {
  const match = stderr.match(/Video:.*?(\d{2,5})x(\d{2,5})/)
  if (!match) return null
  return { width: parseInt(match[1], 10), height: parseInt(match[2], 10) }
}

export function getVideoDurationMs(videoPath: string): Promise<number> {
  return new Promise((resolve, reject) => {
    // ffmpeg -i with no output prints metadata (including Duration) then exits
    const proc = spawn(FFMPEG, ['-i', videoPath])
    let stderr = ''
    proc.stderr.on('data', (chunk: Buffer) => { stderr += chunk.toString() })
    proc.on('close', () => {
      const ms = parseFfmpegDuration(stderr)
      if (ms != null) return resolve(ms)
      reject(new Error('Could not read video duration'))
    })
    proc.on('error', reject)
  })
}

export function getVideoSize(
  videoPath: string
): Promise<{ width: number; height: number }> {
  return new Promise((resolve, reject) => {
    const proc = spawn(FFMPEG, ['-i', videoPath])
    let stderr = ''
    proc.stderr.on('data', (chunk: Buffer) => { stderr += chunk.toString() })
    proc.on('close', () => {
      const size = parseFfmpegVideoSize(stderr)
      if (size) return resolve(size)
      reject(new Error('Could not read video size'))
    })
    proc.on('error', reject)
  })
}

export function buildPreviewClipArgs(
  videoPath: string,
  outputPath: string,
  startSec: number,
  durationSec: number
): string[] {
  return [
    '-ss', String(startSec),
    '-i', videoPath,
    '-t', String(durationSec),
    '-vf', 'scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p',
    '-c:v', 'libx264',
    '-preset', 'veryfast',
    '-crf', '18',
    '-pix_fmt', 'yuv420p',
    '-c:a', 'aac',
    '-b:a', '192k',
    '-threads', '0',
    '-movflags', '+faststart',
    '-avoid_negative_ts', 'make_zero',
    '-y',
    outputPath
  ]
}

export async function extractPreviewClip(
  videoPath: string,
  startMs: number,
  endMs: number,
  onProgress?: (percent: number) => void
): Promise<string> {
  const startSec = Math.max(0, startMs / 1000)
  const durationSec = Math.max(0.2, (endMs - startMs) / 1000)
  const outPath = join(tmpdir(), `a1slice-preview-${randomUUID()}.mp4`)
  try {
    await spawnFfmpeg(
      buildPreviewClipArgs(videoPath, outPath, startSec, durationSec),
      durationSec,
      onProgress
    )
  } catch {
    await spawnFfmpeg(
      [
        '-ss', String(startSec),
        '-i', videoPath,
        '-t', String(durationSec),
        '-vf', 'scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p',
        '-c:v', 'mpeg4',
        '-q:v', '5',
        '-pix_fmt', 'yuv420p',
        '-c:a', 'aac',
        '-b:a', '192k',
        '-movflags', '+faststart',
        '-avoid_negative_ts', 'make_zero',
        '-y',
        outPath
      ],
      durationSec,
      onProgress
    )
  }
  return outPath
}

export function buildCutClipArgs(
  videoPath: string,
  outputPath: string,
  startSec: number,
  durationSec: number,
  videoFilter?: string
): string[] {
  const args = [
    '-ss', String(startSec),
    '-i', videoPath,
    '-t', String(durationSec)
  ]
  if (videoFilter) {
    args.push('-vf', videoFilter)
  }
  args.push(
    '-c:v', 'libx264',
    '-preset', 'veryfast',
    '-crf', '18',
    '-c:a', 'aac',
    '-movflags', '+faststart',
    '-avoid_negative_ts', 'make_zero',
    '-y',
    outputPath
  )
  return args
}

export function buildCopyClipArgs(
  videoPath: string,
  outputPath: string,
  startSec: number,
  durationSec: number
): string[] {
  return [
    '-ss', String(startSec),
    '-i', videoPath,
    '-t', String(durationSec),
    '-c', 'copy',
    '-avoid_negative_ts', 'make_zero',
    '-y',
    outputPath
  ]
}

export async function cutClip(
  videoPath: string,
  outputPath: string,
  startMs: number,
  endMs: number,
  subtitles: TranscriptSegment[],
  onProgress?: (percent: number) => void,
  crop?: ClipCrop,
  videoSize?: { width: number; height: number }
): Promise<string> {
  const startSec = startMs / 1000
  const durationSec = (endMs - startMs) / 1000
  const videoFilter =
    crop && videoSize
      ? ffmpegCropFilter(crop, videoSize.width, videoSize.height) ?? undefined
      : undefined

  try {
    await spawnFfmpeg(
      buildCutClipArgs(videoPath, outputPath, startSec, durationSec, videoFilter),
      durationSec,
      onProgress
    )
  } catch {
    // Some ffmpeg-static builds may lack libx264; keep export working.
    await spawnFfmpeg(
      buildCopyClipArgs(videoPath, outputPath, startSec, durationSec),
      durationSec,
      onProgress
    )
  }

  // Save SRT with subtitles shifted relative to clip start
  const shifted = subtitles
    .filter((s) => s.endMs > startMs && s.startMs < endMs)
    .map((s) => ({
      startMs: Math.max(0, s.startMs - startMs),
      endMs: Math.min(endMs - startMs, s.endMs - startMs),
      text: s.text
    }))
  const srtPath = outputPath.replace(/\.[^.]+$/, '.srt')
  writeFileSync(srtPath, generateSrt(shifted), 'utf-8')

  return outputPath
}
