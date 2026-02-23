import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron'
import { join, basename } from 'path'
import { mkdirSync, existsSync, unlinkSync } from 'fs'
import { extractAudio, cutClipWithSubtitles } from './ffmpeg'
import { downloadModel, transcribe } from './whisper'
import { analyzeTranscript } from './analyzer'
import { loadSettings, saveSettings } from './settings'
import type { AppSettings, ProgressUpdate } from '../shared/types'

let mainWindow: BrowserWindow | null = null
let cancelled = false

function sendProgress(update: ProgressUpdate): void {
  mainWindow?.webContents.send('pipeline-progress', update)
}

function createWindow(): void {
  const isMac = process.platform === 'darwin'

  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    minWidth: 600,
    minHeight: 400,
    frame: isMac ? true : false,
    titleBarStyle: isMac ? 'hiddenInset' : undefined,
    trafficLightPosition: isMac ? { x: 12, y: 12 } : undefined,
    webPreferences: {
      preload: join(__dirname, '../preload/preload.js'),
      sandbox: false
    }
  })

  if (process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

// IPC: Settings
ipcMain.handle('load-settings', () => {
  return loadSettings()
})

ipcMain.handle('save-settings', (_event, settings: AppSettings) => {
  saveSettings(settings)
})

// IPC: Window controls
ipcMain.handle('window-minimize', () => {
  mainWindow?.minimize()
})

ipcMain.handle('window-maximize', () => {
  if (mainWindow?.isMaximized()) {
    mainWindow.unmaximize()
  } else {
    mainWindow?.maximize()
  }
})

ipcMain.handle('window-close', () => {
  mainWindow?.close()
})

// IPC: Select video file
ipcMain.handle('select-video', async () => {
  if (!mainWindow) return null
  const result = await dialog.showOpenDialog(mainWindow, {
    filters: [
      { name: 'Video Files', extensions: ['mp4', 'mov', 'avi', 'mkv', 'webm'] }
    ],
    properties: ['openFile']
  })
  return result.canceled ? null : result.filePaths[0]
})

// IPC: Open folder in native file manager
ipcMain.handle('open-folder', async (_event, folderPath: string) => {
  await shell.openPath(folderPath)
})

// IPC: Cancel pipeline
ipcMain.on('cancel-pipeline', () => {
  cancelled = true
})

// IPC: Run full pipeline
ipcMain.handle(
  'run-pipeline',
  async (_event, videoPath: string, settings: AppSettings) => {
    cancelled = false

    try {
      // Step 1: Extract audio
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

      // Step 2: Download whisper model if needed
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

      // Step 3: Transcribe
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

      // Step 4: Analyze transcript with LLM
      sendProgress({
        stage: 'analyzing',
        message: 'AI is picking the best clips...',
        percent: 0
      })
      const clips = await analyzeTranscript(
        segments,
        settings.provider,
        settings.model,
        settings.apiKey
      )

      if (cancelled) throw new Error('Cancelled')

      // Step 5: Cut clips with subtitles
      const videoName = basename(videoPath, '.mp4').replace(/\.[^.]+$/, '')
      const outputDir = join(
        app.getPath('videos'),
        `a1slice-${videoName}-${Date.now()}`
      )
      if (!existsSync(outputDir)) mkdirSync(outputDir, { recursive: true })

      for (let i = 0; i < clips.length; i++) {
        if (cancelled) throw new Error('Cancelled')

        const clip = clips[i]
        const safeTitle = clip.title.replace(/[^a-zA-Z0-9 _-]/g, '').slice(0, 50)
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

        await cutClipWithSubtitles(
          videoPath,
          outputPath,
          clip.startMs,
          clip.endMs,
          segments
        )
      }

      sendProgress({
        stage: 'done',
        message: outputDir,
        percent: 100
      })
    } catch (err) {
      const message =
        err instanceof Error ? err.message : 'Unknown error occurred'
      sendProgress({
        stage: 'error',
        message,
        percent: 0
      })
    }
  }
)

app.whenReady().then(createWindow)

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow()
})
