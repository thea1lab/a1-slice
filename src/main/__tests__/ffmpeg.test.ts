import { spawn } from 'child_process'
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import ffmpegPath from 'ffmpeg-static'
import { describe, it, expect } from 'vitest'
import { formatSrtTime, generateSrt, parseFfmpegDuration, parseFfmpegProgress, parseFfmpegVideoSize, buildCutClipArgs, buildPreviewClipArgs, captionBurnFilter, cutClip, findCaptionFont, fontCellRatio, videoFilterForExport } from '../ffmpeg'
import { DEFAULT_CAPTION_STYLE } from '../../shared/captions'

describe('formatSrtTime', () => {
  it('formats zero', () => {
    expect(formatSrtTime(0)).toBe('00:00:00,000')
  })

  it('formats milliseconds only', () => {
    expect(formatSrtTime(500)).toBe('00:00:00,500')
  })

  it('formats seconds and millis', () => {
    expect(formatSrtTime(5123)).toBe('00:00:05,123')
  })

  it('formats minutes', () => {
    expect(formatSrtTime(90000)).toBe('00:01:30,000')
  })

  it('formats hours', () => {
    expect(formatSrtTime(3661001)).toBe('01:01:01,001')
  })
})

describe('generateSrt', () => {
  it('returns empty string for no segments', () => {
    expect(generateSrt([])).toBe('')
  })

  it('generates valid SRT for one segment', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 2000, text: 'Hello world' }
    ])
    expect(result).toBe('1\n00:00:00,000 --> 00:00:02,000\nHello world\n')
  })

  it('generates valid SRT for multiple segments', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 2000, text: 'First' },
      { startMs: 2000, endMs: 5000, text: 'Second' }
    ])
    expect(result).toBe(
      '1\n00:00:00,000 --> 00:00:02,000\nFirst\n\n2\n00:00:02,000 --> 00:00:05,000\nSecond\n'
    )
  })

  it('trims whitespace from text', () => {
    const result = generateSrt([
      { startMs: 0, endMs: 1000, text: '  spaced  ' }
    ])
    expect(result).toContain('spaced')
    expect(result).not.toContain('  spaced  ')
  })
})

describe('parseFfmpegProgress', () => {
  it('returns the last time= line from accumulated stderr', () => {
    const stderr =
      'frame=1 time=00:00:00.04 bitrate=N/A\nframe=40 time=00:00:03.20 bitrate=N/A\n'
    expect(parseFfmpegProgress(stderr)).toBe(3)
  })

  it('returns null when time= is missing', () => {
    expect(parseFfmpegProgress('frame=1 fps=30')).toBeNull()
  })
})

describe('parseFfmpegDuration', () => {
  it('parses centiseconds as a fraction of a second', () => {
    expect(parseFfmpegDuration('Duration: 00:01:23.45, start: 0.000000')).toBe(
      83450
    )
  })

  it('parses three-digit fractional seconds as milliseconds', () => {
    expect(parseFfmpegDuration('  Duration: 01:00:00.500\n')).toBe(3600500)
  })

  it('returns null when duration is missing', () => {
    expect(parseFfmpegDuration('no duration here')).toBeNull()
  })
})

describe('parseFfmpegVideoSize', () => {
  it('reads width and height from a video stream line', () => {
    const stderr =
      'Stream #0:0(und): Video: h264 (High) (avc1 / 0x31637661), yuv420p, 1920x1080 [SAR 1:1 DAR 16:9]'
    expect(parseFfmpegVideoSize(stderr)).toEqual({ width: 1920, height: 1080 })
  })

  it('returns null when no video size is present', () => {
    expect(parseFfmpegVideoSize('Duration: 00:01:00.00')).toBeNull()
  })
})

describe('buildCutClipArgs', () => {
  it('re-encodes instead of stream-copying so cuts are frame-accurate', () => {
    const args = buildCutClipArgs('/in.mp4', '/out.mp4', 12.5, 30)
    expect(args).not.toContain('copy')
    expect(args).toContain('libx264')
    expect(args).toContain('aac')
    expect(args[args.indexOf('-ss') + 1]).toBe('12.5')
    expect(args[args.indexOf('-t') + 1]).toBe('30')
  })

  it('adds a crop/scale filter when provided', () => {
    const args = buildCutClipArgs(
      '/in.mp4',
      '/out.mp4',
      0,
      5,
      'crop=608:1080:656:0,scale=1080:1920'
    )
    expect(args[args.indexOf('-vf') + 1]).toBe('crop=608:1080:656:0,scale=1080:1920')
  })
})

describe('caption burn filter', () => {
  it('includes the subtitles filter only for a burn look', () => {
    const burn = captionBurnFilter('/tmp/a.srt', '/usr/share/fonts', 'DejaVu Sans', {
      ...DEFAULT_CAPTION_STYLE,
      size: 'large'
    })
    expect(burn).toContain('subtitles=')
    expect(burn).toContain('FontSize=28')
    expect(burn).toContain('Outline=0.55')
    expect(burn).toContain('PrimaryColour=&H00FFFFFF')
    expect(burn).toContain('Alignment=2')
    expect(burn).toContain('MarginV=90')
    const scaled = captionBurnFilter('/tmp/a.srt', '/usr/share/fonts', 'DejaVu Sans', {
      ...DEFAULT_CAPTION_STYLE,
      size: 'large'
    }, 1.618)
    expect(scaled).toContain('FontSize=45')
    expect(scaled).toContain('MarginV=90')
    expect(videoFilterForExport('crop=1:1:0:0', 'burn', burn)).toBe(`crop=1:1:0:0,${burn}`)
    expect(videoFilterForExport('crop=1:1:0:0', 'srt', burn)).toBe('crop=1:1:0:0')
    expect(videoFilterForExport(null, 'off', burn)).toBeUndefined()
  })

  it('writes the chosen colour, place, and size into the filter', () => {
    const burn = captionBurnFilter('/tmp/a.srt', '/fonts', 'Noto Serif', {
      color: 'yellow',
      position: 'top',
      size: 'small',
      font: 'serif'
    })
    expect(burn).toContain('FontName=Noto Serif')
    expect(burn).toContain('FontSize=18')
    expect(burn).toContain('PrimaryColour=&H004AE1FF')
    expect(burn).toContain('Alignment=8')
    expect(burn).toContain('MarginV=36')
  })

  it('prefers a bundled face file over a system font', () => {
    const dir = mkdtempSync(join(tmpdir(), 'a1-fonts-'))
    try {
      writeFileSync(join(dir, 'NotoSerif-Regular.ttf'), '')
      expect(findCaptionFont('serif', [dir])).toEqual({
        dir,
        name: 'Noto Serif',
        file: join(dir, 'NotoSerif-Regular.ttf'),
        cellRatio: 1
      })
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it('prefers the medium face the preview uses at weight 500', () => {
    const dir = mkdtempSync(join(tmpdir(), 'a1-fonts-'))
    try {
      writeFileSync(join(dir, 'NotoSansMono-Regular.ttf'), '')
      writeFileSync(join(dir, 'NotoSansMono-Medium.ttf'), '')
      expect(findCaptionFont('mono', [dir])).toMatchObject({
        name: 'Noto Sans Mono Medium',
        file: join(dir, 'NotoSansMono-Medium.ttf')
      })
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it.skipIf(!existsSync('/usr/share/fonts/noto/NotoSansMono-Medium.ttf'))(
    'reads the Noto Sans Mono cell height',
    () => {
      expect(fontCellRatio('/usr/share/fonts/noto/NotoSansMono-Medium.ttf')).toBeCloseTo(1.618, 3)
      expect(fontCellRatio('/usr/share/fonts/noto/NotoSans-Medium.ttf')).toBeCloseTo(1.519, 3)
    }
  )

  it('writes a subtitle file beside a soft export and not beside a burn', async () => {
    if (!findCaptionFont('sans')) return
    const dir = mkdtempSync(join(tmpdir(), 'a1-burn-'))
    const cue = [{ startMs: 0, endMs: 400, text: 'Hello' }]
    try {
      const src = join(dir, 'in.mp4')
      await runFfmpeg([
        '-f', 'lavfi', '-i', 'color=c=black:s=320x180:d=0.5',
        '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-y', src
      ])
      const burned = join(dir, 'burned.mp4')
      await cutClip(src, burned, 0, 400, cue, undefined, undefined, undefined, 'burn')
      expect(existsSync(burned)).toBe(true)
      expect(existsSync(join(dir, 'burned.srt'))).toBe(false)
      const soft = join(dir, 'soft.mp4')
      await cutClip(src, soft, 0, 400, cue, undefined, undefined, undefined, 'srt')
      expect(existsSync(join(dir, 'soft.srt'))).toBe(true)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })
})

function runFfmpeg(args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(ffmpegPath as string, args)
    let stderr = ''
    proc.stderr.on('data', (chunk: Buffer) => {
      stderr += chunk.toString()
    })
    proc.on('close', (code) => {
      if (code === 0) resolve()
      else reject(new Error(stderr.slice(-400)))
    })
    proc.on('error', reject)
  })
}

describe('buildPreviewClipArgs', () => {
  it('re-encodes at native size into Chromium-safe H.264 4:2:0', () => {
    const args = buildPreviewClipArgs('/in.mp4', '/preview.mp4', 5, 8)
    expect(args).toContain('libx264')
    expect(args).toContain('veryfast')
    expect(args).toContain('yuv420p')
    expect(args[args.indexOf('-vf') + 1]).toBe(
      'scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p'
    )
    expect(args.some((arg) => arg.includes('scale='))).toBe(true)
    expect(args).not.toContain('copy')
    expect(args[args.indexOf('-ss') + 1]).toBe('5')
    expect(args[args.indexOf('-t') + 1]).toBe('8')
  })
})
