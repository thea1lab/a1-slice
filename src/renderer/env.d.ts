import type {
  AppSettings,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  TranscribeResult,
  AnalyzeResult,
  CutResult,
  CheckTranscriptResult
} from '../shared/types'

declare global {
  interface Window {
    api: {
      selectVideo(): Promise<string | null>
      checkTranscript(videoPath: string): Promise<CheckTranscriptResult>
      transcribeVideo(videoPath: string): Promise<TranscribeResult>
      analyzeTranscript(
        segments: TranscriptSegment[],
        settings: AppSettings
      ): Promise<AnalyzeResult>
      cutClips(
        videoPath: string,
        clips: ClipSegment[],
        segments: TranscriptSegment[]
      ): Promise<CutResult>
      cancelPipeline(): void
      onProgress(callback: (update: ProgressUpdate) => void): () => void
      openFolder(folderPath: string): Promise<void>
      loadSettings(): Promise<AppSettings>
      saveSettings(settings: AppSettings): Promise<void>
      windowMinimize(): Promise<void>
      windowClose(): Promise<void>
      getPlatform(): string
    }
  }
}
