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

export type LLMProvider = 'claude' | 'openai' | 'opencode'

export const DEFAULT_MODELS: Record<LLMProvider, string> = {
  claude: 'claude-haiku-4-5',
  openai: 'gpt-5-mini-2025-08-07',
  opencode: 'minimax-m2.7'
}

/** OpenCode Zen gateway (https://opencode.ai/zen). */
export const OPENCODE_ZEN_BASE_URL = 'https://opencode.ai/zen/v1'

export interface OpenCodeModelOption {
  id: string
  label: string
  group: string
}

export const OPENCODE_MODELS: readonly OpenCodeModelOption[] = [
  { id: 'minimax-m2.7', label: 'MiniMax M2.7', group: 'Open models' },
  { id: 'minimax-m3', label: 'MiniMax M3', group: 'Open models' },
  { id: 'minimax-m2.5', label: 'MiniMax M2.5', group: 'Open models' },
  { id: 'glm-5.2', label: 'GLM 5.2', group: 'Open models' },
  { id: 'glm-5.1', label: 'GLM 5.1', group: 'Open models' },
  { id: 'glm-5', label: 'GLM 5', group: 'Open models' },
  { id: 'kimi-k3', label: 'Kimi K3', group: 'Open models' },
  { id: 'kimi-k2.7-code', label: 'Kimi K2.7 Code', group: 'Open models' },
  { id: 'kimi-k2.6', label: 'Kimi K2.6', group: 'Open models' },
  { id: 'kimi-k2.5', label: 'Kimi K2.5', group: 'Open models' },
  { id: 'deepseek-v4-pro', label: 'DeepSeek V4 Pro', group: 'Open models' },
  { id: 'deepseek-v4-flash', label: 'DeepSeek V4 Flash', group: 'Open models' },
  { id: 'claude-haiku-4-5', label: 'Claude Haiku 4.5', group: 'Claude' },
  { id: 'claude-sonnet-4-5', label: 'Claude Sonnet 4.5', group: 'Claude' },
  { id: 'claude-sonnet-4-6', label: 'Claude Sonnet 4.6', group: 'Claude' },
  { id: 'claude-sonnet-5', label: 'Claude Sonnet 5', group: 'Claude' },
  { id: 'claude-opus-4-5', label: 'Claude Opus 4.5', group: 'Claude' },
  { id: 'claude-opus-4-6', label: 'Claude Opus 4.6', group: 'Claude' },
  { id: 'claude-opus-4-7', label: 'Claude Opus 4.7', group: 'Claude' },
  { id: 'claude-opus-4-8', label: 'Claude Opus 4.8', group: 'Claude' },
  { id: 'claude-opus-5', label: 'Claude Opus 5', group: 'Claude' },
  { id: 'claude-fable-5', label: 'Claude Fable 5', group: 'Claude' },
  { id: 'claude-fable-5-1', label: 'Claude Fable 5.1', group: 'Claude' },
  { id: 'gpt-5.4-nano', label: 'GPT 5.4 Nano', group: 'GPT' },
  { id: 'gpt-5.4-mini', label: 'GPT 5.4 Mini', group: 'GPT' },
  { id: 'gpt-5.4', label: 'GPT 5.4', group: 'GPT' },
  { id: 'gpt-5.5', label: 'GPT 5.5', group: 'GPT' },
  { id: 'gpt-5.6-luna', label: 'GPT 5.6 Luna', group: 'GPT' },
  { id: 'gpt-5.6-sol', label: 'GPT 5.6 Sol', group: 'GPT' },
  { id: 'gpt-5.6-terra', label: 'GPT 5.6 Terra', group: 'GPT' },
  { id: 'gpt-5', label: 'GPT 5', group: 'GPT' },
  { id: 'gpt-5-nano', label: 'GPT 5 Nano', group: 'GPT' },
  { id: 'gemini-3-flash', label: 'Gemini 3 Flash', group: 'Gemini' },
  { id: 'gemini-3.5-flash', label: 'Gemini 3.5 Flash', group: 'Gemini' },
  { id: 'gemini-3.5-flash-lite', label: 'Gemini 3.5 Flash Lite', group: 'Gemini' },
  { id: 'gemini-3.1-pro', label: 'Gemini 3.1 Pro', group: 'Gemini' },
  { id: 'gemini-3.6-flash', label: 'Gemini 3.6 Flash', group: 'Gemini' },
  { id: 'gemini-3.7-flash', label: 'Gemini 3.7 Flash', group: 'Gemini' },
  { id: 'gemini-3.8-flash', label: 'Gemini 3.8 Flash', group: 'Gemini' },
  { id: 'grok-build-0.1', label: 'Grok Build 0.1', group: 'Grok' },
  { id: 'grok-4.5', label: 'Grok 4.5', group: 'Grok' },
  { id: 'grok-4.6', label: 'Grok 4.6', group: 'Grok' },
  { id: 'qwen3.5-plus', label: 'Qwen3.5 Plus', group: 'Qwen' },
  { id: 'qwen3.6-plus', label: 'Qwen3.6 Plus', group: 'Qwen' },
  { id: 'big-pickle', label: 'Big Pickle (free)', group: 'Free' },
  { id: 'mimo-v2.5-free', label: 'MiMo V2.5 Free', group: 'Free' },
  { id: 'ling-3.0-flash-fin-free', label: 'Ling 3.0 Flash Fin Free', group: 'Free' },
  { id: 'deepseek-v4-flash-free', label: 'DeepSeek V4 Flash Free', group: 'Free' },
  { id: 'nemotron-3-ultra-free', label: 'Nemotron 3 Ultra Free', group: 'Free' },
  { id: 'nemotron-3.5-lightning-free', label: 'Nemotron 3.5 Lightning Free', group: 'Free' }
]

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
  temperatureInc: number
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
