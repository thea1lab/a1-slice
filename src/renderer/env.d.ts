import type { AppSettings, ProgressUpdate } from '../shared/types'

declare global {
  interface Window {
    api: {
      selectVideo(): Promise<string | null>
      runPipeline(videoPath: string, settings: AppSettings): Promise<void>
      cancelPipeline(): void
      onProgress(callback: (update: ProgressUpdate) => void): () => void
      openFolder(folderPath: string): Promise<void>
      loadSettings(): Promise<AppSettings>
      saveSettings(settings: AppSettings): Promise<void>
      windowMinimize(): Promise<void>
      windowClose(): Promise<void>
      getPlatform(): string
    }
  }
}
