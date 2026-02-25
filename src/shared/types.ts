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

export interface ClipSegment {
  title: string
  startMs: number
  endMs: number
  category?: 'related' | 'standalone'
  topic?: string
}

export interface ProgressUpdate {
  stage: PipelineStage
  message: string
  percent: number
}

export type LLMProvider = 'claude' | 'openai'

export type VideoLanguage = 'auto' | 'en' | 'pt' | 'es'

export interface AppSettings {
  provider: LLMProvider
  model: string
  apiKey: string
  userHint: string
  language: VideoLanguage
  entropyThold: number
  maxContext: number
  beamSize: number
}

// --- Wizard types ---

export type WizardStep =
  | 'select'
  | 'transcribe'
  | 'review-transcript'
  | 'review-slices'
  | 'export'

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

export interface ClipSegmentWithStatus extends ClipSegment {
  id: string
  approved: boolean
}
