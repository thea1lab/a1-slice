import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import type { AppSettings, LLMProvider } from '../shared/types'

const SETTINGS_DIR = join(app.getPath('home'), '.a1slice')
const SETTINGS_PATH = join(SETTINGS_DIR, 'settings.json')

const DEFAULT_SETTINGS: AppSettings = {
  provider: 'claude',
  model: 'claude-haiku-4-5-20251001',
  apiKey: ''
}

const MODEL_MIGRATIONS: Record<string, string> = {
  'claude-sonnet-4-20250514': 'claude-haiku-4-5-20251001',
  'gpt-4o': 'gpt-5-mini-2025-08-07'
}

const PROVIDER_MIGRATIONS: Record<string, LLMProvider> = {
  gpt4o: 'openai'
}

export function loadSettings(): AppSettings {
  try {
    if (existsSync(SETTINGS_PATH)) {
      const raw = readFileSync(SETTINGS_PATH, 'utf-8')
      const parsed = JSON.parse(raw)
      const settings = { ...DEFAULT_SETTINGS, ...parsed }
      if (settings.provider in PROVIDER_MIGRATIONS) {
        settings.provider = PROVIDER_MIGRATIONS[settings.provider]
      }
      if (settings.model in MODEL_MIGRATIONS) {
        settings.model = MODEL_MIGRATIONS[settings.model]
      }
      return settings
    }
  } catch {
    // Fall through to defaults
  }
  return { ...DEFAULT_SETTINGS }
}

export function saveSettings(settings: AppSettings): void {
  if (!existsSync(SETTINGS_DIR)) {
    mkdirSync(SETTINGS_DIR, { recursive: true })
  }
  writeFileSync(SETTINGS_PATH, JSON.stringify(settings, null, 2), 'utf-8')
}
