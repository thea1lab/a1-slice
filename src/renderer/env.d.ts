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

declare global {
  interface Window {
    api: {
      selectVideo(): Promise<string | null>
      checkTranscript(videoPath: string): Promise<CheckTranscriptResult>
      checkAnalysis(videoPath: string): Promise<CheckAnalysisResult>
      transcribeVideo(videoPath: string): Promise<TranscribeResult>
      analyzeTranscript(
        videoPath: string,
        segments: TranscriptSegment[],
        settings: AppSettings,
        userHint?: string
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
