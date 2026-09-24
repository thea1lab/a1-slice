import type {
  AppSettings,
  CaptionLook,
  CaptionProject,
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
        segments: TranscriptSegment[],
        subtitlesMode?: SubtitleExport,
        burnLook?: CaptionLook
      ): Promise<CutResult>
      loadProject(videoPath: string): Promise<ProjectData>
      saveClips(videoPath: string, clips: ClipSegment[], rawResponse?: string): Promise<void>
      saveFraming(videoPath: string, framing: Partial<Record<CropRatio, ClipCrop>>): Promise<void>
      saveCaptions(videoPath: string, captions: CaptionProject): Promise<void>
      rememberVideo(videoPath: string): Promise<void>
      listRecent(): Promise<RecentVideo[]>
      openTranscript(videoPath: string, segments: TranscriptSegment[]): Promise<{ success: boolean; error?: string }>
      exportReframed(videoPath: string, crop: ClipCrop): Promise<CutResult>
      exportCaptions(videoPath: string, cues: TranscriptSegment[], look: CaptionLook): Promise<CutResult>
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
