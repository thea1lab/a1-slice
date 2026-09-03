import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  shell,
  protocol,
  net,
  session
} from 'electron'
import { join, basename, dirname, extname, resolve, normalize } from 'path'
import { pathToFileURL } from 'url'
import { mkdirSync, existsSync, unlinkSync, writeFileSync, readFileSync, statSync } from 'fs'

import { extractAudio, splitWav, cutClip, getVideoDurationMs, extractPreviewClip } from './ffmpeg'
import { downloadModel, transcribeWithRetry, mergeChunkSegments, type AbortHandle } from './whisper'
import { analyzeTranscript, parseLLMResponse, formatTranscriptForLLM } from './analyzer'
import { loadSettings, saveSettings } from './settings'
import { pathFromPreviewUrl } from '../shared/previewUrl'
import type {
  AppSettings,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  ClipPreviewResult
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
        color: '#08080f',
        symbolColor: '#737373',
        height: 40
      }
    }),
    backgroundColor: '#0f0f1a',
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
  async (_event, videoPath: string, startMs: number, endMs: number): Promise<ClipPreviewResult> => {
    if (typeof videoPath !== 'string' || !videoPath) {
      return { success: false, error: 'Missing video path' }
    }
    if (!Number.isFinite(startMs) || !Number.isFinite(endMs) || endMs <= startMs) {
      return { success: false, error: 'Invalid clip range' }
    }
    allowVideoPath(videoPath)
    try {
      const previewPath = await withPreviewSlot(() =>
        extractPreviewClip(videoPath, startMs, endMs)
      )
      allowVideoPath(previewPath)
      previewFiles.add(previewPath)
      return { success: true, previewPath }
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Preview cut failed'
      return { success: false, error: message }
    }
  }
)

secureHandle('release-clip-preview', (_event, previewPath: string) => {
  if (typeof previewPath !== 'string' || !previewPath) return
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
        message: 'Extracting audio from video...',
        percent: 0
      })
      const wavPath = await extractAudio(videoPath, (pct) => {
        sendProgress({
          stage: 'extracting',
          message: 'Extracting audio from video...',
          percent: pct
        })
      })

      if (cancelled) throw new Error('Cancelled')

      // Download whisper model if needed
      sendProgress({
        stage: 'downloading',
        message: 'Checking whisper model...',
        percent: 0
      })
      await downloadModel((pct) => {
        sendProgress({
          stage: 'downloading',
          message: 'Downloading whisper model...',
          percent: pct
        })
      })

      if (cancelled) throw new Error('Cancelled')

      // Split audio into chunks for more reliable transcription
      sendProgress({
        stage: 'transcribing',
        message: 'Splitting audio into chunks...',
        percent: 0
      })
      const overlapSec = 15
      const chunks = await splitWav(wavPath, 180, overlapSec)
      const totalChunks = chunks.length
      const chunkResults: { offsetMs: number; durationMs: number; segments: TranscriptSegment[] }[] = []

      for (let i = 0; i < totalChunks; i++) {
        if (cancelled) throw new Error('Cancelled')

        const chunk = chunks[i]
        const chunkLabel = `Transcribing chunk ${i + 1}/${totalChunks}...`
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

      // Cache transcript next to source video
      try {
        const vName = basename(videoPath).replace(/\.[^.]+$/, '')
        const cachePath = join(dirname(videoPath), `${vName}.a1slice.json`)
        writeFileSync(cachePath, JSON.stringify(allSegments), 'utf-8')
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
  try {
    const vName = basename(videoPath).replace(/\.[^.]+$/, '')
    const cachePath = join(dirname(videoPath), `${vName}.a1slice.json`)
    if (existsSync(cachePath)) {
      const data = readFileSync(cachePath, 'utf-8')
      const segments = JSON.parse(data)
      return { found: true, segments }
    }
  } catch {}
  return { found: false }
})

// IPC: Check for cached analysis
secureHandle('check-analysis', (_event, videoPath: string) => {
  allowVideoPath(videoPath)
  try {
    const vName = basename(videoPath).replace(/\.[^.]+$/, '')
    const dir = dirname(videoPath)
    const clipsPath = join(dir, `${vName}.a1slice-clips.json`)
    const analysisPath = join(dir, `${vName}.a1slice-analysis.txt`)

    // Try new clips JSON cache first
    if (existsSync(clipsPath)) {
      const clipsData = JSON.parse(readFileSync(clipsPath, 'utf-8'))
      const clips: ClipSegment[] = (clipsData as any[]).map((item: any) => {
        const clip: ClipSegment = {
          title: item.title,
          startMs: item.startMs,
          endMs: item.endMs
        }
        if (item.category === 'related' || item.category === 'standalone') {
          clip.category = item.category
        }
        if (item.topic) {
          clip.topic = item.topic
        }
        return clip
      })
      const rawResponse = existsSync(analysisPath)
        ? readFileSync(analysisPath, 'utf-8')
        : ''
      return { found: true, clips, rawResponse }
    }

    // Backward compat: old .a1slice-analysis.txt format
    if (existsSync(analysisPath)) {
      const rawResponse = readFileSync(analysisPath, 'utf-8')
      const clips = parseLLMResponse(rawResponse)
      return { found: true, clips, rawResponse }
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
        message: 'AI is picking the best clips...',
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

      // Cache analysis next to source video
      try {
        const vName = basename(videoPath).replace(/\.[^.]+$/, '')
        const dir = dirname(videoPath)
        // Save clips as structured JSON for reliable re-parsing
        writeFileSync(
          join(dir, `${vName}.a1slice-clips.json`),
          JSON.stringify(clips),
          'utf-8'
        )
        // Save full debug output for inspection
        writeFileSync(
          join(dir, `${vName}.a1slice-analysis.txt`),
          rawResponse,
          'utf-8'
        )
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
    segments: TranscriptSegment[]
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

        await cutClip(
          videoPath,
          outputPath,
          clampedStartMs,
          clampedEndMs,
          segments
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

  // Handle a1slice:// protocol for video preview (with range request support for seeking)
  protocol.handle('a1slice', (req) => {
    const rawPath = pathFromPreviewUrl(req.url)
    if (!rawPath) return new Response('Missing path', { status: 400 })

    const filePath = resolve(normalize(rawPath))
    const ext = extname(filePath).toLowerCase()
    if (!VIDEO_EXTENSIONS.has(ext) || !isAllowedVideoPath(filePath)) {
      return new Response('Forbidden', { status: 403 })
    }

    try {
      statSync(filePath)
    } catch {
      return new Response('File not found', { status: 404 })
    }

    const range = req.headers.get('Range') ?? req.headers.get('range')
    return net.fetch(pathToFileURL(filePath).href, {
      ...(range ? { headers: { Range: range } } : {}),
      bypassCustomProtocolHandlers: true
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
