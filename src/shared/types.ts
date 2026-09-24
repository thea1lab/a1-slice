export type PipelineStage =
  | 'idle'
  | 'extracting'
  | 'downloading'
  | 'transcribing'
  | 'analyzing'
  | 'cutting'
  | 'done'
  | 'error'

export interface TranscriptSegment {
  startMs: number
  endMs: number
  text: string
}

export type CropRatio = 'original' | '16:9' | '4:3' | '9:16' | '1:1'

export interface ClipCrop {
  ratio: CropRatio
  /** Center of the crop window in the source frame, 0–1. */
  cx: number
  cy: number
  /** 0 = largest window that fits the ratio, 1 = punched in. */
  zoom: number
}

export const DEFAULT_CROP: ClipCrop = {
  ratio: 'original',
  cx: 0.5,
  cy: 0.5,
  zoom: 0
}

export interface ClipSegment {
  title: string
  startMs: number
  endMs: number
  category?: 'related' | 'standalone'
  topic?: string
  crop?: ClipCrop
  /** Missing on older cache files. Treated as kept. */
  approved?: boolean
}

export interface ProgressUpdate {
  stage: PipelineStage
  message: string
  percent: number
}

export type LLMProvider = 'claude' | 'openai' | 'opencode'

export const DEFAULT_MODELS: Record<LLMProvider, string> = {
  claude: 'claude-haiku-4-5',
  openai: 'gpt-5-mini-2025-08-07',
  opencode: 'minimax-m2.7'
}

/** OpenCode Zen gateway (https://opencode.ai/zen). */
export const OPENCODE_ZEN_BASE_URL = 'https://opencode.ai/zen/v1'

export type VideoLanguage = 'auto' | 'en' | 'pt' | 'es'

export interface AppSettings {
  provider: LLMProvider
  model: string
  apiKey: string
  /** Keys stored per provider so switching providers can restore them. */
  apiKeys: Record<string, string>
  userHint: string
  language: VideoLanguage
  entropyThold: number
  maxContext: number
  beamSize: number
  temperatureInc: number
}

// --- Screens ---

export type ToolId = 'transcribe' | 'find' | 'reframe' | 'captions'

export type Screen =
  | 'home'
  | 'transcribe'
  | 'transcribe-done'
  | 'find'
  | 'review'
  | 'export'
  | 'reframe'
  | 'captions'

export type SubtitleExport = 'off' | 'srt' | 'burn'

export type CaptionLook = 'srt' | 'burn-large' | 'burn-small'

export type CaptionSource = 'transcript' | 'manual'

export interface CaptionProject {
  source: CaptionSource
  look: CaptionLook
  cues: TranscriptSegment[]
}

export interface VideoInspection {
  hasTranscript: boolean
  segmentCount: number
  hasClips: boolean
  clipCount: number
  hasFraming: boolean
  hasCaptions: boolean
}

export interface RecentVideo extends VideoInspection {
  path: string
}

export interface ProjectData {
  segments: TranscriptSegment[]
  clips: ClipSegment[]
  rawResponse: string
  framing: Partial<Record<CropRatio, ClipCrop>>
  captions: CaptionProject | null
  durationMs: number
}

export interface TranscribeResult {
  success: boolean
  segments?: TranscriptSegment[]
  error?: string
}

export interface AnalyzeResult {
  success: boolean
  clips?: ClipSegment[]
  rawResponse?: string
  error?: string
}

export interface CheckTranscriptResult {
  found: boolean
  segments?: TranscriptSegment[]
}

export interface CheckAnalysisResult {
  found: boolean
  clips?: ClipSegment[]
  rawResponse?: string
}

export interface CutResult {
  success: boolean
  outputDir?: string
  error?: string
}

export interface ClipPreviewResult {
  success: boolean
  previewPath?: string
  cached?: boolean
  error?: string
}

export interface ClipSegmentWithStatus extends ClipSegment {
  id: string
  approved: boolean
}
