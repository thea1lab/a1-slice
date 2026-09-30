import { contextBridge, ipcRenderer } from 'electron'
import type { CaptionEditRequest } from '../shared/captionEdit'
import type {
  AppSettings,
  CaptionFilePick,
  CaptionLook,
  CaptionProject,
  CaptionStyle,
  ClipCrop,
  CropRatio,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  TranscribeResult,
  AnalyzeResult,
  CutResult,
  ClipPreviewResult,
  CheckTranscriptResult,
  CheckAnalysisResult,
  ProjectData,
  RecentVideo,
  SubtitleExport
} from '../shared/types'

contextBridge.exposeInMainWorld('api', {
  selectVideo: (): Promise<string | null> =>
    ipcRenderer.invoke('select-video'),

  selectCaptionFile: (): Promise<CaptionFilePick | null> =>
    ipcRenderer.invoke('select-caption-file'),

  allowVideoPath: (videoPath: string): Promise<void> =>
    ipcRenderer.invoke('allow-video-path', videoPath),

  createClipPreview: (
    videoPath: string,
    startMs: number,
    endMs: number,
    previewId?: string
  ): Promise<ClipPreviewResult> =>
    ipcRenderer.invoke('create-clip-preview', videoPath, startMs, endMs, previewId),

  releaseClipPreview: (previewPath: string): Promise<void> =>
    ipcRenderer.invoke('release-clip-preview', previewPath),

  readClipPreview: (previewPath: string): Promise<Uint8Array | null> =>
    ipcRenderer.invoke('read-clip-preview', previewPath),

  transcribeVideo: (videoPath: string, language?: string, entropyThold?: number, maxContext?: number, beamSize?: number, temperatureInc?: number): Promise<TranscribeResult> =>
    ipcRenderer.invoke('transcribe-video', videoPath, language, entropyThold, maxContext, beamSize, temperatureInc),

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
    segments: TranscriptSegment[],
    subtitlesMode?: SubtitleExport,
    burnStyle?: CaptionStyle
  ): Promise<CutResult> =>
    ipcRenderer.invoke('cut-clips', videoPath, clips, segments, subtitlesMode, burnStyle),

  loadProject: (videoPath: string): Promise<ProjectData> =>
    ipcRenderer.invoke('load-project', videoPath),

  saveClips: (videoPath: string, clips: ClipSegment[], rawResponse?: string): Promise<void> =>
    ipcRenderer.invoke('save-clips', videoPath, clips, rawResponse),

  saveFraming: (
    videoPath: string,
    framing: Partial<Record<CropRatio, ClipCrop>>
  ): Promise<void> => ipcRenderer.invoke('save-framing', videoPath, framing),

  saveCaptions: (videoPath: string, captions: CaptionProject): Promise<void> =>
    ipcRenderer.invoke('save-captions', videoPath, captions),

  rememberVideo: (videoPath: string): Promise<void> =>
    ipcRenderer.invoke('remember-video', videoPath),

  listRecent: (): Promise<RecentVideo[]> => ipcRenderer.invoke('list-recent'),

  openTranscript: (
    videoPath: string,
    segments: TranscriptSegment[]
  ): Promise<{ success: boolean; error?: string }> =>
    ipcRenderer.invoke('open-transcript', videoPath, segments),

  exportReframed: (videoPath: string, crop: ClipCrop): Promise<CutResult> =>
    ipcRenderer.invoke('export-reframed', videoPath, crop),

  exportCaptions: (
    videoPath: string,
    cues: TranscriptSegment[],
    look: CaptionLook,
    style: CaptionStyle
  ): Promise<CutResult> => ipcRenderer.invoke('export-captions', videoPath, cues, look, style),

  listCaptionAgents: (): Promise<{ id: string; label: string }[]> =>
    ipcRenderer.invoke('list-caption-agents'),

  fixCaptions: (
    agentId: string,
    segments: TranscriptSegment[],
    request: CaptionEditRequest
  ): Promise<{ success: boolean; segments?: TranscriptSegment[]; error?: string }> =>
    ipcRenderer.invoke('fix-captions', agentId, segments, request),

  cancelCaptionFix: (): void => {
    ipcRenderer.send('cancel-caption-fix')
  },

  saveTranscript: (
    videoPath: string,
    segments: TranscriptSegment[]
  ): Promise<{ success: boolean; path?: string; error?: string }> =>
    ipcRenderer.invoke('save-transcript', videoPath, segments),

  cancelPipeline: (): void => {
    ipcRenderer.send('cancel-pipeline')
  },

  onCaptionFixLog: (callback: (line: string) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, line: string): void => {
      callback(line)
    }
    ipcRenderer.on('caption-fix-log', handler)
    return () => {
      ipcRenderer.removeListener('caption-fix-log', handler)
    }
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

  onPreviewProgress: (
    callback: (update: { previewId?: string; percent: number }) => void
  ): (() => void) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      update: { previewId?: string; percent: number } | number
    ): void => {
      if (typeof update === 'number') {
        callback({ percent: update })
        return
      }
      callback(update)
    }
    ipcRenderer.on('preview-progress', handler)
    return () => {
      ipcRenderer.removeListener('preview-progress', handler)
    }
  },

  openFolder: (folderPath: string): Promise<void> =>
    ipcRenderer.invoke('open-folder', folderPath),

  loadSettings: (): Promise<AppSettings> =>
    ipcRenderer.invoke('load-settings'),

  saveSettings: (settings: AppSettings): Promise<void> =>
    ipcRenderer.invoke('save-settings', settings),

  getPlatform: (): string => process.platform
})
