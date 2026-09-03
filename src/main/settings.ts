import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import type { AppSettings, LLMProvider } from '../shared/types'

const SETTINGS_DIR = join(app.getPath('home'), '.a1slice')
const SETTINGS_PATH = join(SETTINGS_DIR, 'settings.json')

const DEFAULT_SETTINGS: AppSettings = {
  provider: 'claude',
  model: 'claude-haiku-4-5',
  apiKey: '',
  apiKeys: {},
  userHint: '',
  language: 'auto',
  entropyThold: 2.8,
  maxContext: 64,
  beamSize: 5,
  temperatureInc: 0.1
}

export function loadSettings(): AppSettings {
  try {
    if (existsSync(SETTINGS_PATH)) {
      const raw = readFileSync(SETTINGS_PATH, 'utf-8')
      const parsed = JSON.parse(raw) as Partial<AppSettings>
      const apiKeys = migrateApiKeys(
        { ...DEFAULT_SETTINGS.apiKeys, ...(parsed.apiKeys ?? {}) },
        mergedProvider(parsed),
        parsed.apiKey ?? ''
      )
      const merged: AppSettings = { ...DEFAULT_SETTINGS, ...parsed, apiKeys }
      merged.apiKey = merged.apiKeys[merged.provider] ?? merged.apiKey ?? ''
      return merged
    }
  } catch {
    // Fall through to defaults
  }
  return { ...DEFAULT_SETTINGS, apiKeys: {} }
}

function mergedProvider(parsed: Partial<AppSettings>): LLMProvider {
  return parsed.provider ?? DEFAULT_SETTINGS.provider
}

/** Prefer `provider` keys; copy legacy `provider:model` slots if needed. */
function migrateApiKeys(
  apiKeys: Record<string, string>,
  provider: LLMProvider,
  apiKey: string
): Record<string, string> {
  const next = { ...apiKeys }
  if (!next[provider]) {
    if (apiKey) {
      next[provider] = apiKey
    } else {
      const legacy = Object.entries(apiKeys).find(
        ([id]) => id === provider || id.startsWith(`${provider}:`)
      )
      if (legacy) next[provider] = legacy[1]
    }
  }
  return next
}

export function saveSettings(settings: AppSettings): void {
  if (!existsSync(SETTINGS_DIR)) {
    mkdirSync(SETTINGS_DIR, { recursive: true })
  }
  writeFileSync(SETTINGS_PATH, JSON.stringify(settings, null, 2), 'utf-8')
}
