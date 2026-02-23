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
}

export interface ProgressUpdate {
  stage: PipelineStage
  message: string
  percent: number
}

export type LLMProvider = 'claude' | 'openai'

export interface AppSettings {
  provider: LLMProvider
  model: string
  apiKey: string
}
