import type {
  AppSettings,
  ProgressUpdate,
  TranscriptSegment,
  ClipSegment,
  TranscribeResult,
  AnalyzeResult,
  CutResult,
  ClipPreviewResult,
  CheckTranscriptResult,
  CheckAnalysisResult
} from '../shared/types'

declare global {
  interface Window {
    api: {
      selectVideo(): Promise<string | null>
      allowVideoPath(videoPath: string): Promise<void>
      createClipPreview(videoPath: string, startMs: number, endMs: number, previewId?: string): Promise<ClipPreviewResult>
      releaseClipPreview(previewPath: string): Promise<void>
      readClipPreview(previewPath: string): Promise<Uint8Array | null>
      checkTranscript(videoPath: string): Promise<CheckTranscriptResult>
      checkAnalysis(videoPath: string): Promise<CheckAnalysisResult>
      transcribeVideo(videoPath: string, language?: string, entropyThold?: number, maxContext?: number, beamSize?: number, temperatureInc?: number): Promise<TranscribeResult>
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
      onPreviewProgress(callback: (update: { previewId?: string; percent: number }) => void): () => void
      openFolder(folderPath: string): Promise<void>
      loadSettings(): Promise<AppSettings>
      saveSettings(settings: AppSettings): Promise<void>
      getPlatform(): string
    }
  }
}
