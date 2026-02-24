import { contextBridge, ipcRenderer } from 'electron'
import type {
  AppSettings,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  TranscribeResult,
  AnalyzeResult,
  CutResult,
  CheckTranscriptResult,
  CheckAnalysisResult
} from '../shared/types'

contextBridge.exposeInMainWorld('api', {
  selectVideo: (): Promise<string | null> =>
    ipcRenderer.invoke('select-video'),

  transcribeVideo: (videoPath: string, language?: string): Promise<TranscribeResult> =>
    ipcRenderer.invoke('transcribe-video', videoPath, language),

  analyzeTranscript: (
    videoPath: string,
    segments: TranscriptSegment[],
    settings: AppSettings,
    userHint?: string
  ): Promise<AnalyzeResult> =>
    ipcRenderer.invoke('analyze-transcript', videoPath, segments, settings, userHint),

  checkTranscript: (videoPath: string): Promise<CheckTranscriptResult> =>
    ipcRenderer.invoke('check-transcript', videoPath),

  checkAnalysis: (videoPath: string): Promise<CheckAnalysisResult> =>
    ipcRenderer.invoke('check-analysis', videoPath),

  cutClips: (
    videoPath: string,
    clips: ClipSegment[],
    segments: TranscriptSegment[]
  ): Promise<CutResult> =>
    ipcRenderer.invoke('cut-clips', videoPath, clips, segments),

  cancelPipeline: (): void => {
    ipcRenderer.send('cancel-pipeline')
  },

  onProgress: (callback: (update: ProgressUpdate) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, update: ProgressUpdate): void => {
      callback(update)
    }
    ipcRenderer.on('pipeline-progress', handler)
    return () => {
      ipcRenderer.removeListener('pipeline-progress', handler)
    }
  },

  openFolder: (folderPath: string): Promise<void> =>
    ipcRenderer.invoke('open-folder', folderPath),

  loadSettings: (): Promise<AppSettings> =>
    ipcRenderer.invoke('load-settings'),

  saveSettings: (settings: AppSettings): Promise<void> =>
    ipcRenderer.invoke('save-settings', settings),

  windowMinimize: (): Promise<void> =>
    ipcRenderer.invoke('window-minimize'),

  windowClose: (): Promise<void> =>
    ipcRenderer.invoke('window-close'),

  getPlatform: (): string => process.platform
})
