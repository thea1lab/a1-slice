import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs'
import { join } from 'path'
import { app } from 'electron'
import type { AppSettings } from '../shared/types'

const SETTINGS_DIR = join(app.getPath('home'), '.a1slice')
const SETTINGS_PATH = join(SETTINGS_DIR, 'settings.json')

const DEFAULT_SETTINGS: AppSettings = {
  provider: 'claude',
  model: 'claude-haiku-4-5',
  apiKey: '',
  userHint: '',
  language: 'auto',
  entropyThold: 2.4,
  noContext: false
}

export function loadSettings(): AppSettings {
  try {
    if (existsSync(SETTINGS_PATH)) {
      const raw = readFileSync(SETTINGS_PATH, 'utf-8')
      const parsed = JSON.parse(raw)
      return { ...DEFAULT_SETTINGS, ...parsed }
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
