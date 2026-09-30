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

declare global {
  interface Window {
    api: {
      selectVideo(): Promise<string | null>
      selectCaptionFile(): Promise<CaptionFilePick | null>
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
        burnStyle?: CaptionStyle
      ): Promise<CutResult>
      loadProject(videoPath: string): Promise<ProjectData>
      saveClips(videoPath: string, clips: ClipSegment[], rawResponse?: string): Promise<void>
      saveFraming(videoPath: string, framing: Partial<Record<CropRatio, ClipCrop>>): Promise<void>
      saveCaptions(videoPath: string, captions: CaptionProject): Promise<void>
      rememberVideo(videoPath: string): Promise<void>
      listRecent(): Promise<RecentVideo[]>
      openTranscript(videoPath: string, segments: TranscriptSegment[]): Promise<{ success: boolean; error?: string }>
      exportReframed(videoPath: string, crop: ClipCrop): Promise<CutResult>
      exportCaptions(
        videoPath: string,
        cues: TranscriptSegment[],
        look: CaptionLook,
        style: CaptionStyle
      ): Promise<CutResult>
      listCaptionAgents(): Promise<{ id: string; label: string }[]>
      fixCaptions(
        agentId: string,
        segments: TranscriptSegment[],
        request: CaptionEditRequest
      ): Promise<{ success: boolean; segments?: TranscriptSegment[]; error?: string }>
      cancelCaptionFix(): void
      saveTranscript(
        videoPath: string,
        segments: TranscriptSegment[]
      ): Promise<{ success: boolean; path?: string; error?: string }>
      onCaptionFixLog(callback: (line: string) => void): () => void
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
