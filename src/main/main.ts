import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  shell,
  protocol,
  session
} from 'electron'
import { join, basename, dirname, extname, resolve, normalize } from 'path'
import { mkdirSync, existsSync, unlinkSync, writeFileSync, readFileSync, statSync, createReadStream, copyFileSync } from 'fs'
import { Readable } from 'stream'

import { extractAudio, splitWav, cutClip, copyClip, getVideoDurationMs, getVideoSize, extractPreviewClip, generateSrt } from './ffmpeg'
import { ffmpegCropFilter } from '../shared/crop'
import { downloadModel, transcribeWithRetry, mergeChunkSegments, type AbortHandle } from './whisper'
import { analyzeTranscript, parseLLMResponse, formatTranscriptForLLM } from './analyzer'
import { loadSettings, saveSettings } from './settings'
import { pathFromPreviewUrl } from '../shared/previewUrl'
import { previewCacheKey } from '../shared/project'
import { parseByteRange, rangeResponseMeta, videoMimeForExt } from './httpRange'
import {
  cachedPreviewPath,
  inspectVideo,
  isCachedPreview,
  previewCacheDir,
  readProjectFiles,
  readRecent,
  videoStem,
  writeCaptions,
  writeClips,
  writeFraming,
  writeRemembered,
  writeTranscript
} from './projectStore'
import type {
  AppSettings,
  CaptionLook,
  CaptionProject,
  ClipCrop,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  ClipPreviewResult,
  CropRatio,
  SubtitleExport
} from '../shared/types'

app.commandLine.appendSwitch('ignore-gpu-blocklist')
app.commandLine.appendSwitch('log-level', '3')

// Register custom protocol before app is ready
protocol.registerSchemesAsPrivileged([
  {
    scheme: 'a1slice',
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
      stream: true,
      bypassCSP: true,
      corsEnabled: true
    }
  }
])

let mainWindow: BrowserWindow | null = null
let cancelled = false
let activeWhisperHandle: AbortHandle | null = null
const allowedVideoPaths = new Set<string>()
const previewFiles = new Set<string>()
let previewRunning = 0
const previewWaiters: Array<() => void> = []

const VIDEO_EXTENSIONS = new Set(['.mp4', '.mov', '.mkv', '.avi', '.webm'])

function allowVideoPath(videoPath: string): string {
  const filePath = resolve(normalize(videoPath))
  allowedVideoPaths.add(filePath)
  allowedVideoPaths.add(normalize(videoPath))
  return filePath
}

function isAllowedVideoPath(filePath: string): boolean {
  if (allowedVideoPaths.has(filePath) || allowedVideoPaths.has(normalize(filePath))) {
    return true
  }
  if (process.platform !== 'win32') return false
  const lower = filePath.toLowerCase()
  for (const allowed of allowedVideoPaths) {
    if (allowed.toLowerCase() === lower) return true
  }
  return false
}

function secureHandle(
  channel: string,
  handler: (event: Electron.IpcMainInvokeEvent, ...args: any[]) => any
): void {
  ipcMain.handle(channel, (event, ...args) => {
    if (event.sender !== mainWindow?.webContents) return undefined
    return handler(event, ...args)
  })
}

function sendProgress(update: ProgressUpdate): void {
  mainWindow?.webContents.send('pipeline-progress', update)
}

function createWindow(): void {
  const isMac = process.platform === 'darwin'

  mainWindow = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 900,
    minHeight: 600,
    resizable: true,
    frame: true,
    titleBarStyle: isMac ? 'hiddenInset' : 'hidden',
    trafficLightPosition: isMac ? { x: 12, y: 12 } : undefined,
    ...(isMac ? {} : {
      titleBarOverlay: {
        color: '#161616',
        symbolColor: '#f4f1ea',
        height: 48
      }
    }),
    backgroundColor: '#161616',
    maximizable: true,
    autoHideMenuBar: !isMac,
    icon: join(__dirname, '../../resources/icon.png'),
    webPreferences: {
      preload: join(__dirname, '../preload/preload.js'),
      sandbox: true,
      contextIsolation: true
    }
  })

  if (process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  if (!isMac) {
    mainWindow.setMenuBarVisibility(false)
  }

  // Block navigation to external URLs
  mainWindow.webContents.on('will-navigate', (event, url) => {
    const devUrl = process.env['ELECTRON_RENDERER_URL']
    if (devUrl && url.startsWith(devUrl)) return
    event.preventDefault()
  })

  // Block popups
  mainWindow.webContents.setWindowOpenHandler(() => ({ action: 'deny' }))
}

// IPC: Settings
secureHandle('load-settings', () => {
  return loadSettings()
})

secureHandle('save-settings', (_event, settings: AppSettings) => {
  saveSettings(settings)
})

// IPC: Select video file
secureHandle('select-video', async () => {
  if (!mainWindow) return null
  const result = await dialog.showOpenDialog(mainWindow, {
    filters: [
      { name: 'Video Files', extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm'] }
    ],
    properties: ['openFile']
  })
  if (result.canceled) return null
  const selected = result.filePaths[0]
  allowVideoPath(selected)
  return selected
})

// IPC: Open folder in native file manager
secureHandle('open-folder', async (_event, folderPath: string) => {
  shell.showItemInFolder(folderPath)
})

secureHandle('allow-video-path', (_event, videoPath: string) => {
  if (typeof videoPath !== 'string' || !videoPath) return
  const ext = extname(videoPath).toLowerCase()
  if (!VIDEO_EXTENSIONS.has(ext)) return
  allowVideoPath(videoPath)
})

async function withPreviewSlot<T>(fn: () => Promise<T>): Promise<T> {
  if (previewRunning >= 2) {
    await new Promise<void>((resolve) => previewWaiters.push(resolve))
  }
  previewRunning++
  try {
    return await fn()
  } finally {
    previewRunning--
    previewWaiters.shift()?.()
  }
}

secureHandle(
  'create-clip-preview',
  async (
    _event,
    videoPath: string,
    startMs: number,
    endMs: number,
    previewId?: string
  ): Promise<ClipPreviewResult> => {
    if (typeof videoPath !== 'string' || !videoPath) {
      return { success: false, error: 'Missing video path' }
    }
    if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || endMs <= startMs) {
      return { success: false, error: 'Invalid clip range' }
    }
    allowVideoPath(videoPath)
    try {
      const sourceStat = statSync(videoPath)
      const key = previewCacheKey(
        videoPath,
        sourceStat.size,
        sourceStat.mtimeMs,
        startMs,
        endMs
      )
      const cachePath = cachedPreviewPath(key)
      if (existsSync(cachePath) && statSync(cachePath).size > 0) {
        allowVideoPath(cachePath)
        return { success: true, previewPath: cachePath, cached: true }
      }

      const previewPath = await withPreviewSlot(() =>
        extractPreviewClip(videoPath, startMs, endMs, (percent) => {
          mainWindow?.webContents.send('preview-progress', {
            previewId: typeof previewId === 'string' ? previewId : undefined,
            percent
          })
        })
      )
      try {
        if (!existsSync(previewCacheDir())) mkdirSync(previewCacheDir(), { recursive: true })
        copyFileSync(previewPath, cachePath)
        unlinkSync(previewPath)
        allowVideoPath(cachePath)
        return { success: true, previewPath: cachePath, cached: false }
      } catch {
        allowVideoPath(previewPath)
        previewFiles.add(previewPath)
        return { success: true, previewPath, cached: false }
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Preview cut failed'
      return { success: false, error: message }
    }
  }
)

secureHandle('release-clip-preview', (_event, previewPath: string) => {
  if (typeof previewPath !== 'string' || !previewPath) return
  if (isCachedPreview(previewPath)) return
  if (!previewFiles.has(previewPath)) return
  previewFiles.delete(previewPath)
  try {
    unlinkSync(previewPath)
  } catch {
    // already gone
  }
})

secureHandle('read-clip-preview', (_event, previewPath: string): Uint8Array | null => {
  if (typeof previewPath !== 'string' || !previewPath) return null
  if (!previewFiles.has(previewPath)) return null
  try {
    return readFileSync(previewPath)
  } catch {
    return null
  }
})

// IPC: Cancel pipeline
ipcMain.on('cancel-pipeline', (event) => {
  if (event.sender !== mainWindow?.webContents) return
  cancelled = true
  activeWhisperHandle?.kill()
  activeWhisperHandle = null
})

// IPC: Transcribe video (Step 2)
secureHandle(
  'transcribe-video',
  async (_event, videoPath: string, language?: string, entropyThold?: number, maxContext?: number, beamSize?: number, temperatureInc?: number) => {
    cancelled = false
    allowVideoPath(videoPath)

    try {
      // Extract audio
      sendProgress({
        stage: 'extracting',
        message: 'Reading the audio…',
        percent: 0
      })
      const wavPath = await extractAudio(videoPath, (pct) => {
        sendProgress({
          stage: 'extracting',
          message: 'Reading the audio…',
          percent: pct
        })
      })

      if (cancelled) throw new Error('Cancelled')

      // Download whisper model if needed
      sendProgress({
        stage: 'downloading',
        message: 'Checking the speech model…',
        percent: 0
      })
      await downloadModel((pct) => {
        sendProgress({
          stage: 'downloading',
          message: 'Downloading the speech model…',
          percent: pct
        })
      })

      if (cancelled) throw new Error('Cancelled')

      // Split audio into chunks for more reliable transcription
      sendProgress({
        stage: 'transcribing',
        message: 'Preparing the audio…',
        percent: 0
      })
      const overlapSec = 15
      const chunks = await splitWav(wavPath, 180, overlapSec)
      const totalChunks = chunks.length
      const chunkResults: { offsetMs: number; durationMs: number; segments: TranscriptSegment[] }[] = []

      for (let i = 0; i < totalChunks; i++) {
        if (cancelled) throw new Error('Cancelled')

        const chunk = chunks[i]
        const chunkLabel =
          totalChunks === 1
            ? 'Writing the transcript…'
            : `Writing the transcript, part ${i + 1} of ${totalChunks}…`
        sendProgress({
          stage: 'transcribing',
          message: chunkLabel,
          percent: Math.round((i / totalChunks) * 100)
        })

        const handle = transcribeWithRetry(chunk.path, {
          language,
          entropyThold,
          maxContext,
          beamSize,
          temperatureInc,
          timeoutMs: 10 * 60 * 1000, // 10 minutes per chunk
          onProgress: (pct) => {
            const overallPct = Math.round(((i + pct / 100) / totalChunks) * 100)
            sendProgress({
              stage: 'transcribing',
              message: chunkLabel,
              percent: overallPct
            })
          }
        })
        activeWhisperHandle = handle

        try {
          const chunkSegments = await handle.promise
          chunkResults.push({
            offsetMs: chunk.offsetMs,
            durationMs: chunk.durationMs,
            segments: chunkSegments
          })
        } finally {
          activeWhisperHandle = null
        }

        // Clean up chunk file
        try { unlinkSync(chunk.path) } catch {}
      }

      const allSegments = mergeChunkSegments(chunkResults, overlapSec * 1000)

      // Clean up full WAV
      try { unlinkSync(wavPath) } catch {}

      if (cancelled) throw new Error('Cancelled')

      try {
        writeTranscript(videoPath, allSegments)
      } catch {}

      sendProgress({ stage: 'done', message: '', percent: 100 })
      return { success: true, segments: allSegments }
    } catch (err) {
      if (cancelled) {
        sendProgress({ stage: 'error', message: 'Cancelled', percent: 0 })
        return { success: false, error: 'Cancelled' }
      }
      const message =
        err instanceof Error ? err.message : 'Unknown error occurred'
      sendProgress({ stage: 'error', message, percent: 0 })
      return { success: false, error: message }
    }
  }
)

// IPC: Check for cached transcript
secureHandle('check-transcript', (_event, videoPath: string) => {
  allowVideoPath(videoPath)
  const segments = readProjectFiles(videoPath).segments
  if (segments.length > 0) return { found: true, segments }
  return { found: false }
})

// IPC: Check for cached analysis
secureHandle('check-analysis', (_event, videoPath: string) => {
  allowVideoPath(videoPath)
  try {
    const { clips, rawResponse } = readProjectFiles(videoPath)
    if (clips.length > 0) return { found: true, clips, rawResponse }
    const { dir, stem } = videoStem(videoPath)
    const analysisPath = join(dir, `${stem}.a1slice-analysis.txt`)
    if (existsSync(analysisPath)) {
      const legacy = readFileSync(analysisPath, 'utf-8')
      return { found: true, clips: parseLLMResponse(legacy), rawResponse: legacy }
    }
  } catch {}
  return { found: false }
})

// IPC: Analyze transcript with LLM (Step 3)
secureHandle(
  'analyze-transcript',
  async (
    _event,
    videoPath: string,
    segments: TranscriptSegment[],
    settings: AppSettings,
    userHint?: string
  ) => {
    cancelled = false
    allowVideoPath(videoPath)

    try {
      sendProgress({
        stage: 'analyzing',
        message: 'Reading the transcript and marking clips…',
        percent: 0
      })

      const { clips, rawResponse } = await analyzeTranscript(
        segments,
        settings.provider,
        settings.model,
        settings.apiKey,
        userHint,
        (message, percent) => sendProgress({ stage: 'analyzing', message, percent })
      )

      if (cancelled) throw new Error('Cancelled')

      try {
        writeClips(videoPath, clips, rawResponse)
      } catch {}

      sendProgress({ stage: 'done', message: '', percent: 100 })
      return { success: true, clips, rawResponse }
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Unknown error occurred'
      sendProgress({ stage: 'error', message, percent: 0 })
      return { success: false, error: message }
    }
  }
)

// IPC: Cut clips with subtitles (Step 5)
secureHandle(
  'cut-clips',
  async (
    _event,
    videoPath: string,
    clips: ClipSegment[],
    segments: TranscriptSegment[],
    subtitlesMode?: SubtitleExport,
    burnLook?: CaptionLook
  ) => {
    cancelled = false
    allowVideoPath(videoPath)

    try {
      const videoName = basename(videoPath).replace(/\.[^.]+$/, '')
      const outputDir = join(
        dirname(videoPath),
        `a1slice-${videoName}-${Date.now()}`
      )
      if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true })

      // Get video duration to clamp clip times
      let videoDurationMs = Infinity
      try {
        videoDurationMs = await getVideoDurationMs(videoPath)
      } catch {}

      let videoSize: { width: number; height: number } | undefined
      try {
        videoSize = await getVideoSize(videoPath)
      } catch {}

      // Save transcript as a text file in the output directory
      const transcriptPath = join(outputDir, 'transcript.txt')
      writeFileSync(transcriptPath, formatTranscriptForLLM(segments), 'utf-8')

      for (let i = 0; i < clips.length; i++) {
        if (cancelled) throw new Error('Cancelled')

        const clip = clips[i]
        const clampedStartMs = Math.min(clip.startMs, videoDurationMs)
        const clampedEndMs = Math.min(clip.endMs, videoDurationMs)
        if (clampedEndMs <= clampedStartMs) continue

        const safeTitle = clip.title
          .replace(/[^a-zA-Z0-9 _-]/g, '')
          .slice(0, 50)
        const outputPath = join(
          outputDir,
          `${String(i + 1).padStart(2, '0')}_${safeTitle}.mp4`
        )
        const overallPercent = Math.round(((i + 1) / clips.length) * 100)

        sendProgress({
          stage: 'cutting',
          message: `Cutting clip ${i + 1}/${clips.length}: ${clip.title}`,
          percent: overallPercent
        })

        const mode: SubtitleExport =
          subtitlesMode === 'off' || subtitlesMode === 'burn' || subtitlesMode === 'srt'
            ? subtitlesMode
            : 'srt'
        const look = burnLook === 'burn-small' ? 'burn-small' : 'burn-large'
        await cutClip(
          videoPath,
          outputPath,
          clampedStartMs,
          clampedEndMs,
          segments,
          undefined,
          clip.crop,
          videoSize,
          mode,
          look,
          captionFontDirs()
        )
      }

      sendProgress({ stage: 'done', message: outputDir, percent: 100 })
      return { success: true, outputDir }
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Unknown error occurred'
      sendProgress({ stage: 'error', message, percent: 0 })
      return { success: false, error: message }
    }
  }
)

function captionFontDirs(): string[] {
  return [
    join(process.resourcesPath || '', 'fonts'),
    join(__dirname, '../../resources/fonts')
  ]
}

secureHandle('load-project', async (_event, videoPath: string) => {
  if (typeof videoPath !== 'string' || !videoPath) {
    return {
      segments: [],
      clips: [],
      rawResponse: '',
      framing: {},
      captions: null,
      durationMs: 0
    }
  }
  allowVideoPath(videoPath)
  const project = readProjectFiles(videoPath)
  let durationMs = 0
  try {
    durationMs = await getVideoDurationMs(videoPath)
  } catch {
    durationMs = 0
  }
  return { ...project, durationMs }
})

secureHandle('save-clips', (_event, videoPath: string, clips: ClipSegment[], rawResponse?: string) => {
  if (typeof videoPath !== 'string' || !videoPath || !Array.isArray(clips)) return
  allowVideoPath(videoPath)
  writeClips(videoPath, clips, rawResponse)
})

secureHandle(
  'save-framing',
  (_event, videoPath: string, framing: Partial<Record<CropRatio, ClipCrop>>) => {
    if (typeof videoPath !== 'string' || !videoPath || !framing) return
    allowVideoPath(videoPath)
    writeFraming(videoPath, framing)
  }
)

secureHandle('save-captions', (_event, videoPath: string, captions: CaptionProject) => {
  if (typeof videoPath !== 'string' || !videoPath || !captions) return
  allowVideoPath(videoPath)
  writeCaptions(videoPath, captions)
})

secureHandle('remember-video', (_event, videoPath: string) => {
  if (typeof videoPath !== 'string' || !videoPath) return
  allowVideoPath(videoPath)
  writeRemembered(videoPath)
})

secureHandle('list-recent', () => {
  return readRecent()
    .filter((path) => existsSync(path))
    .map((path) => ({ path, ...inspectVideo(path) }))
})

secureHandle('open-transcript', async (_event, videoPath: string, segments: TranscriptSegment[]) => {
  if (typeof videoPath !== 'string' || !videoPath || !Array.isArray(segments)) {
    return { success: false, error: 'Missing transcript' }
  }
  allowVideoPath(videoPath)
  try {
    const txtPath = writeTranscript(videoPath, segments)
    const error = await shell.openPath(txtPath)
    if (error) return { success: false, error }
    return { success: true }
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Could not open the transcript'
    return { success: false, error: message }
  }
})

secureHandle('export-reframed', async (_event, videoPath: string, crop: ClipCrop) => {
  if (typeof videoPath !== 'string' || !videoPath || !crop) {
    return { success: false, error: 'Missing video' }
  }
  allowVideoPath(videoPath)
  cancelled = false
  try {
    const durationMs = await getVideoDurationMs(videoPath)
    let videoSize: { width: number; height: number } | undefined
    try {
      videoSize = await getVideoSize(videoPath)
    } catch {
      videoSize = undefined
    }
    const { dir, stem } = videoStem(videoPath)
    const outputDir = join(dir, `a1slice-${stem}-reframe-${Date.now()}`)
    mkdirSync(outputDir, { recursive: true })
    const outputPath = join(outputDir, `${stem}.mp4`)
    const filter = videoSize ? ffmpegCropFilter(crop, videoSize.width, videoSize.height) : null
    sendProgress({ stage: 'cutting', message: 'Rendering the new frame', percent: 5 })
    const onProgress = (percent: number): void => {
      sendProgress({ stage: 'cutting', message: 'Rendering the new frame', percent })
    }
    if (!filter) {
      await copyClip(videoPath, outputPath, 0, durationMs, onProgress)
    } else {
      await cutClip(
        videoPath,
        outputPath,
        0,
        durationMs,
        [],
        onProgress,
        crop,
        videoSize,
        'off',
        'burn-large',
        captionFontDirs()
      )
    }
    if (cancelled) throw new Error('Cancelled')
    sendProgress({ stage: 'done', message: outputDir, percent: 100 })
    return { success: true, outputDir }
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Reframe failed'
    sendProgress({ stage: 'error', message, percent: 0 })
    return { success: false, error: message }
  }
})

secureHandle(
  'export-captions',
  async (
    _event,
    videoPath: string,
    cues: TranscriptSegment[],
    look: CaptionLook
  ) => {
    if (typeof videoPath !== 'string' || !videoPath || !Array.isArray(cues)) {
      return { success: false, error: 'Missing captions' }
    }
    allowVideoPath(videoPath)
    cancelled = false
    try {
      const { dir, stem } = videoStem(videoPath)
      if (look === 'srt') {
        const srtPath = join(dir, `${stem}.srt`)
        writeFileSync(srtPath, generateSrt(cues), 'utf-8')
        return { success: true, outputDir: srtPath }
      }
      const durationMs = await getVideoDurationMs(videoPath)
      let videoSize: { width: number; height: number } | undefined
      try {
        videoSize = await getVideoSize(videoPath)
      } catch {
        videoSize = undefined
      }
      const outputDir = join(dir, `a1slice-${stem}-captions-${Date.now()}`)
      mkdirSync(outputDir, { recursive: true })
      const outputPath = join(outputDir, `${stem}.mp4`)
      sendProgress({ stage: 'cutting', message: 'Burning captions', percent: 5 })
      await cutClip(
        videoPath,
        outputPath,
        0,
        durationMs,
        cues,
        (percent) => sendProgress({ stage: 'cutting', message: 'Burning captions', percent }),
        undefined,
        videoSize,
        'burn',
        look === 'burn-small' ? 'burn-small' : 'burn-large',
        captionFontDirs()
      )
      if (cancelled) throw new Error('Cancelled')
      sendProgress({ stage: 'done', message: outputDir, percent: 100 })
      return { success: true, outputDir }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Caption export failed'
      sendProgress({ stage: 'error', message, percent: 0 })
      return { success: false, error: message }
    }
  }
)

function applyContentSecurityPolicy(): void {
  const isDev = Boolean(process.env['ELECTRON_RENDERER_URL'])
  const policy = isDev
    ? "default-src 'self' 'unsafe-inline' 'unsafe-eval' http://localhost:* ws://localhost:*; media-src 'self' blob: a1slice:; connect-src 'self' blob: a1slice: http://localhost:* ws://localhost:*; img-src 'self' data:; font-src 'self' data:"
    : "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; media-src 'self' blob: a1slice:; connect-src 'self' blob: a1slice:; img-src 'self' data:; font-src 'self' data:"

  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    const headers = { ...details.responseHeaders }
    for (const key of Object.keys(headers)) {
      if (key.toLowerCase() === 'content-security-policy') delete headers[key]
    }
    headers['Content-Security-Policy'] = [policy]
    callback({ responseHeaders: headers })
  })
}

app.whenReady().then(() => {
  applyContentSecurityPolicy()

  // Serve preview files ourselves so Range requests return 206.
  // Chromium will not seek (currentTime stays 0) without Accept-Ranges + Content-Range.
  protocol.handle('a1slice', (req) => {
    const rawPath = pathFromPreviewUrl(req.url)
    if (!rawPath) return new Response('Missing path', { status: 400 })

    const filePath = resolve(normalize(rawPath))
    const ext = extname(filePath).toLowerCase()
    if (!VIDEO_EXTENSIONS.has(ext) || !isAllowedVideoPath(filePath)) {
      return new Response('Forbidden', { status: 403 })
    }

    let size: number
    try {
      size = statSync(filePath).size
    } catch {
      return new Response('File not found', { status: 404 })
    }

    const rangeHeader = req.headers.get('Range') ?? req.headers.get('range')
    const range = parseByteRange(rangeHeader, size)
    if (rangeHeader && !range) {
      return new Response(null, {
        status: 416,
        headers: {
          'Content-Range': `bytes */${size}`,
          'Accept-Ranges': 'bytes'
        }
      })
    }

    const mime = videoMimeForExt(ext)
    const meta = rangeResponseMeta(size, range, mime)
    const stream = range
      ? createReadStream(filePath, { start: range.start, end: range.end })
      : createReadStream(filePath)
    return new Response(Readable.toWeb(stream) as unknown as ReadableStream, {
      status: meta.status,
      headers: meta.headers
    })
  })

  createWindow()
})

app.on('before-quit', () => {
  for (const previewPath of previewFiles) {
    try {
      unlinkSync(previewPath)
    } catch {
      // ignore
    }
  }
  previewFiles.clear()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow()
})
