import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  shell,
  protocol
} from 'electron'
import { join, basename, dirname, extname, resolve, normalize } from 'path'
import { mkdirSync, existsSync, unlinkSync, writeFileSync, readFileSync, statSync, createReadStream } from 'fs'
import { Readable } from 'stream'

function mimeForVideo(filePath: string): string {
  const ext = extname(filePath).toLowerCase()
  const map: Record<string, string> = {
    '.mp4': 'video/mp4',
    '.mov': 'video/quicktime',
    '.mkv': 'video/x-matroska',
    '.avi': 'video/x-msvideo',
    '.webm': 'video/webm'
  }
  return map[ext] || 'video/mp4'
}
import { extractAudio, cutClip, getVideoDurationMs } from './ffmpeg'
import { downloadModel, transcribe } from './whisper'
import { analyzeTranscript, parseLLMResponse, formatTranscriptForLLM } from './analyzer'
import { loadSettings, saveSettings } from './settings'
import type {
  AppSettings,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment
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
      stream: true
    }
  }
])

let mainWindow: BrowserWindow | null = null
let cancelled = false
const allowedVideoPaths = new Set<string>()

const VIDEO_EXTENSIONS = new Set(['.mp4', '.mov', '.mkv', '.avi', '.webm'])

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
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    resizable: true,
    frame: isMac,
    titleBarStyle: isMac ? 'hiddenInset' : undefined,
    trafficLightPosition: isMac ? { x: 12, y: 12 } : undefined,
    backgroundColor: '#0f0f1a',
    maximizable: true,
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

  // Content Security Policy
  mainWindow.webContents.session.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          process.env['ELECTRON_RENDERER_URL']
            ? "default-src 'self' 'unsafe-inline' 'unsafe-eval' http://localhost:* ws://localhost:*; media-src 'self' a1slice:; connect-src 'self' a1slice: http://localhost:* ws://localhost:*; img-src 'self' data:"
            : "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; media-src 'self' a1slice:; connect-src 'self' a1slice:; img-src 'self' data:; font-src 'self'"
        ]
      }
    })
  })

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

// IPC: Window controls
secureHandle('window-minimize', () => {
  mainWindow?.minimize()
})

secureHandle('window-close', () => {
  mainWindow?.close()
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
  allowedVideoPaths.add(normalize(selected))
  return selected
})

// IPC: Open folder in native file manager
secureHandle('open-folder', async (_event, folderPath: string) => {
  shell.showItemInFolder(folderPath)
})

// IPC: Cancel pipeline
ipcMain.on('cancel-pipeline', (event) => {
  if (event.sender !== mainWindow?.webContents) return
  cancelled = true
})

// IPC: Transcribe video (Step 2)
secureHandle(
  'transcribe-video',
  async (_event, videoPath: string) => {
    cancelled = false

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

      // Transcribe
      sendProgress({
        stage: 'transcribing',
        message: 'Transcribing audio...',
        percent: 0
      })
      const segments = await transcribe(wavPath, (pct) => {
        sendProgress({
          stage: 'transcribing',
          message: 'Transcribing audio...',
          percent: pct
        })
      })

      // Clean up temp WAV
      try {
        unlinkSync(wavPath)
      } catch {}

      if (cancelled) throw new Error('Cancelled')

      // Cache transcript next to source video
      try {
        const vName = basename(videoPath).replace(/\.[^.]+$/, '')
        const cachePath = join(dirname(videoPath), `${vName}.a1slice.json`)
        writeFileSync(cachePath, JSON.stringify(segments), 'utf-8')
      } catch {}

      sendProgress({ stage: 'done', message: '', percent: 100 })
      return { success: true, segments }
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Unknown error occurred'
      sendProgress({ stage: 'error', message, percent: 0 })
      return { success: false, error: message }
    }
  }
)

// IPC: Check for cached transcript
secureHandle('check-transcript', (_event, videoPath: string) => {
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
  try {
    const vName = basename(videoPath).replace(/\.[^.]+$/, '')
    const cachePath = join(dirname(videoPath), `${vName}.a1slice-analysis.txt`)
    if (existsSync(cachePath)) {
      const rawResponse = readFileSync(cachePath, 'utf-8')
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
        userHint
      )

      if (cancelled) throw new Error('Cancelled')

      // Cache analysis next to source video
      try {
        const vName = basename(videoPath).replace(/\.[^.]+$/, '')
        const cachePath = join(dirname(videoPath), `${vName}.a1slice-analysis.txt`)
        writeFileSync(cachePath, rawResponse, 'utf-8')
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

app.whenReady().then(() => {
  // Handle a1slice:// protocol for video preview (with range request support for seeking)
  protocol.handle('a1slice', (req) => {
    const url = new URL(req.url)
    const rawPath = decodeURIComponent(url.searchParams.get('path') || '')
    if (!rawPath) return new Response('Missing path', { status: 400 })

    const filePath = resolve(normalize(rawPath))
    const ext = extname(filePath).toLowerCase()
    if (!VIDEO_EXTENSIONS.has(ext) || !allowedVideoPaths.has(normalize(filePath))) {
      return new Response('Forbidden', { status: 403 })
    }

    let size: number
    try {
      size = statSync(filePath).size
    } catch {
      return new Response('File not found', { status: 404 })
    }

    const mime = mimeForVideo(filePath)
    const range = req.headers.get('range')

    if (range) {
      const m = range.match(/bytes=(\d+)-(\d*)/)
      if (m) {
        const start = parseInt(m[1], 10)
        const end = m[2] ? parseInt(m[2], 10) : size - 1
        return new Response(
          Readable.toWeb(createReadStream(filePath, { start, end, highWaterMark: 256 * 1024 })) as ReadableStream,
          {
            status: 206,
            headers: {
              'Content-Range': `bytes ${start}-${end}/${size}`,
              'Accept-Ranges': 'bytes',
              'Content-Length': String(end - start + 1),
              'Content-Type': mime
            }
          }
        )
      }
    }

    return new Response(
      Readable.toWeb(createReadStream(filePath, { highWaterMark: 256 * 1024 })) as ReadableStream,
      {
        headers: {
          'Accept-Ranges': 'bytes',
          'Content-Length': String(size),
          'Content-Type': mime
        }
      }
    )
  })

  createWindow()
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow()
})
