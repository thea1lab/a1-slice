import ffmpegPath from 'ffmpeg-static'
import { spawn } from 'child_process'
import { tmpdir } from 'os'
import { randomUUID } from 'crypto'
import { existsSync, readFileSync, unlinkSync, writeFileSync } from 'fs'
import { dirname, join } from 'path'
import type { CaptionFont, CaptionStyle, ClipCrop, SubtitleExport, TranscriptSegment } from '../shared/types'
import { captionForceStyle, DEFAULT_CAPTION_STYLE } from '../shared/captions'
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

export function escapeFilterPath(filePath: string): string {
  return filePath.replace(/\\/g, '/').replace(/:/g, '\\:').replace(/'/g, '\\\'')
}

export function captionBurnFilter(
  srtPath: string,
  fontsDir: string,
  fontName: string,
  style: CaptionStyle = DEFAULT_CAPTION_STYLE,
  cellRatio = 1
): string {
  const force = captionForceStyle(style, fontName, cellRatio).replace(/,/g, '\\,')
  return `subtitles='${escapeFilterPath(srtPath)}':fontsdir='${escapeFilterPath(fontsDir)}':force_style='${force}'`
}

export function videoFilterForExport(
  cropFilter: string | null,
  mode: SubtitleExport,
  burnFilter: string | null
): string | undefined {
  const burn = mode === 'burn' ? burnFilter : null
  if (cropFilter && burn) return `${cropFilter},${burn}`
  return burn ?? cropFilter ?? undefined
}

interface FontFile {
  file: string
  name: string
}

const SANS_FILES: FontFile[] = [
  { file: '/usr/share/fonts/noto/NotoSans-Medium.ttf', name: 'Noto Sans Medium' },
  { file: '/usr/share/fonts/noto/NotoSans-Regular.ttf', name: 'Noto Sans' },
  { file: '/usr/share/fonts/liberation/LiberationSans-Regular.ttf', name: 'Liberation Sans' },
  { file: '/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf', name: 'Liberation Sans' },
  { file: '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', name: 'DejaVu Sans' },
  { file: '/usr/share/fonts/truetype/freefont/FreeSans.ttf', name: 'FreeSans' },
  { file: '/System/Library/Fonts/Supplemental/Arial.ttf', name: 'Arial' },
  { file: '/Library/Fonts/Arial.ttf', name: 'Arial' },
  { file: 'C:\\Windows\\Fonts\\arial.ttf', name: 'Arial' }
]

const SERIF_FILES: FontFile[] = [
  { file: '/usr/share/fonts/noto/NotoSerif-Medium.ttf', name: 'Noto Serif Medium' },
  { file: '/usr/share/fonts/noto/NotoSerif-Regular.ttf', name: 'Noto Serif' },
  { file: '/usr/share/fonts/liberation/LiberationSerif-Regular.ttf', name: 'Liberation Serif' },
  { file: '/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf', name: 'Liberation Serif' },
  { file: '/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf', name: 'DejaVu Serif' },
  { file: '/usr/share/fonts/truetype/freefont/FreeSerif.ttf', name: 'FreeSerif' },
  { file: '/System/Library/Fonts/Supplemental/Times New Roman.ttf', name: 'Times New Roman' },
  { file: 'C:\\Windows\\Fonts\\times.ttf', name: 'Times New Roman' }
]

const MONO_FILES: FontFile[] = [
  { file: '/usr/share/fonts/noto/NotoSansMono-Medium.ttf', name: 'Noto Sans Mono Medium' },
  { file: '/usr/share/fonts/noto/NotoSansMono-Regular.ttf', name: 'Noto Sans Mono' },
  { file: '/usr/share/fonts/liberation/LiberationMono-Regular.ttf', name: 'Liberation Mono' },
  { file: '/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf', name: 'Liberation Mono' },
  { file: '/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf', name: 'DejaVu Sans Mono' },
  { file: '/usr/share/fonts/truetype/freefont/FreeMono.ttf', name: 'FreeMono' },
  { file: '/System/Library/Fonts/Supplemental/Courier New.ttf', name: 'Courier New' },
  { file: 'C:\\Windows\\Fonts\\consola.ttf', name: 'Consolas' }
]

const SYSTEM_FONTS: Record<CaptionFont, FontFile[]> = {
  sans: SANS_FILES,
  serif: SERIF_FILES,
  mono: MONO_FILES
}

/** CSS weight 500. The Medium file's family name is what libass matches. */
const BUNDLED_FACES: Record<CaptionFont, FontFile[]> = {
  sans: [
    { file: 'NotoSans-Medium.ttf', name: 'Noto Sans Medium' },
    { file: 'NotoSans-Regular.ttf', name: 'Noto Sans' }
  ],
  serif: [
    { file: 'NotoSerif-Medium.ttf', name: 'Noto Serif Medium' },
    { file: 'NotoSerif-Regular.ttf', name: 'Noto Serif' }
  ],
  mono: [
    { file: 'NotoSansMono-Medium.ttf', name: 'Noto Sans Mono Medium' },
    { file: 'NotoSansMono-Regular.ttf', name: 'Noto Sans Mono' }
  ]
}

function bundledFont(face: CaptionFont, extraDirs: string[]): FontFile[] {
  return extraDirs
    .filter(Boolean)
    .flatMap((dir) => BUNDLED_FACES[face].map((faceFile) => ({ file: join(dir, faceFile.file), name: faceFile.name })))
}

export interface CaptionFontMatch {
  dir: string
  name: string
  file: string
  /** (usWinAscent + usWinDescent) / unitsPerEm. 1 when the face could not be read. */
  cellRatio: number
}

export function findCaptionFont(
  face: CaptionFont = 'sans',
  extraDirs: string[] = []
): CaptionFontMatch | null {
  const order = [
    ...bundledFont(face, extraDirs),
    ...SYSTEM_FONTS[face],
    ...bundledFont('sans', extraDirs),
    ...SYSTEM_FONTS.sans
  ]
  for (const candidate of order) {
    if (candidate.file && existsSync(candidate.file)) {
      return {
        dir: dirname(candidate.file),
        name: candidate.name,
        file: candidate.file,
        cellRatio: fontCellRatio(candidate.file)
      }
    }
  }
  return null
}

/** libass sizes by the Windows cell. CSS sizes by the em. Returns 1 if the face cannot be read. */
export function fontCellRatio(filePath: string): number {
  try {
    return cellRatioFromSfnt(readFileSync(filePath))
  } catch {
    return 1
  }
}

function cellRatioFromSfnt(data: Buffer): number {
  if (data.length < 12) return 1
  const scaler = data.readUInt32BE(0)
  const truetype = 0x00010000
  const trueTag = 0x74727565
  const otto = 0x4f54544f
  if (scaler !== truetype && scaler !== trueTag && scaler !== otto) return 1
  const numTables = data.readUInt16BE(4)
  let head: { offset: number; length: number } | null = null
  let os2: { offset: number; length: number } | null = null
  for (let i = 0; i < numTables; i++) {
    const rec = 12 + i * 16
    if (rec + 16 > data.length) return 1
    const tag = data.toString('latin1', rec, rec + 4)
    const offset = data.readUInt32BE(rec + 8)
    const length = data.readUInt32BE(rec + 12)
    if (tag === 'head') head = { offset, length }
    if (tag === 'OS/2') os2 = { offset, length }
  }
  if (!head || !os2 || head.length < 20 || os2.length < 78) return 1
  if (head.offset + 20 > data.length || os2.offset + 78 > data.length) return 1
  const unitsPerEm = data.readUInt16BE(head.offset + 18)
  const winAscent = data.readUInt16BE(os2.offset + 74)
  const winDescent = data.readUInt16BE(os2.offset + 76)
  if (unitsPerEm === 0) return 1
  const ratio = (winAscent + winDescent) / unitsPerEm
  if (!Number.isFinite(ratio) || ratio < 0.5 || ratio > 2.5) return 1
  return ratio
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

export function shiftSubtitles(
  subtitles: TranscriptSegment[],
  startMs: number,
  endMs: number
): TranscriptSegment[] {
  return subtitles
    .filter((segment) => segment.endMs > startMs && segment.startMs < endMs)
    .map((segment) => ({
      startMs: Math.max(0, segment.startMs - startMs),
      endMs: Math.min(endMs - startMs, segment.endMs - startMs),
      text: segment.text
    }))
}

export async function copyClip(
  videoPath: string,
  outputPath: string,
  startMs: number,
  endMs: number,
  onProgress?: (percent: number) => void
): Promise<void> {
  const startSec = startMs / 1000
  const durationSec = Math.max(0.2, (endMs - startMs) / 1000)
  await spawnFfmpeg(
    buildCopyClipArgs(videoPath, outputPath, startSec, durationSec),
    durationSec,
    onProgress
  )
}

export async function cutClip(
  videoPath: string,
  outputPath: string,
  startMs: number,
  endMs: number,
  subtitles: TranscriptSegment[],
  onProgress?: (percent: number) => void,
  crop?: ClipCrop,
  videoSize?: { width: number; height: number },
  subtitlesMode: SubtitleExport = 'srt',
  burnStyle: CaptionStyle = DEFAULT_CAPTION_STYLE,
  fontDirs: string[] = []
): Promise<string> {
  const startSec = startMs / 1000
  const durationSec = (endMs - startMs) / 1000
  const cropFilter =
    crop && videoSize ? ffmpegCropFilter(crop, videoSize.width, videoSize.height) : null

  const shifted = shiftSubtitles(subtitles, startMs, endMs)
  let burnFilter: string | null = null
  // A sibling .srt is loaded by VLC and mpv on top of a burned caption.
  let temporarySrt: string | null = null
  try {
    if (subtitlesMode === 'srt') {
      const srtPath = outputPath.replace(/\.[^.]+$/, '.srt')
      writeFileSync(srtPath, generateSrt(shifted), 'utf-8')
    } else if (subtitlesMode === 'burn' && shifted.length > 0) {
      temporarySrt = join(tmpdir(), `a1slice-${randomUUID()}.srt`)
      writeFileSync(temporarySrt, generateSrt(shifted), 'utf-8')
      const font = findCaptionFont(burnStyle.font, fontDirs)
      if (!font) throw new Error('No caption font found on this computer')
      burnFilter = captionBurnFilter(temporarySrt, font.dir, font.name, burnStyle, font.cellRatio)
    }

    const videoFilter = videoFilterForExport(cropFilter, subtitlesMode, burnFilter)

    try {
      await spawnFfmpeg(
        buildCutClipArgs(videoPath, outputPath, startSec, durationSec, videoFilter),
        durationSec,
        onProgress
      )
    } catch (err) {
      if (subtitlesMode === 'burn') throw err
      // Some ffmpeg-static builds may lack libx264; keep export working.
      await spawnFfmpeg(
        buildCopyClipArgs(videoPath, outputPath, startSec, durationSec),
        durationSec,
        onProgress
      )
    }

    return outputPath
  } finally {
    if (temporarySrt) {
      try {
        unlinkSync(temporarySrt)
      } catch {
        // The temp subtitle is only an ffmpeg input.
      }
    }
  }
}
