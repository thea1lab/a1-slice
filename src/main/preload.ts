import { contextBridge, ipcRenderer } from 'electron'
import type { AppSettings, ProgressUpdate } from '../shared/types'

contextBridge.exposeInMainWorld('api', {
  selectVideo: (): Promise<string | null> =>
    ipcRenderer.invoke('select-video'),

  runPipeline: (videoPath: string, settings: AppSettings): Promise<void> =>
    ipcRenderer.invoke('run-pipeline', videoPath, settings),

  cancelPipeline: (): void => {
    ipcRenderer.send('cancel-pipeline')
  },

  onProgress: (callback: (update: ProgressUpdate) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, update: ProgressUpdate): void => {
      callback(update)
    }
    ipcRenderer.on('pipeline-progress', handler)
    return () => {
      ipcRenderer.removeListener('pipeline-progress', handler)
    }
  },

  openFolder: (folderPath: string): Promise<void> =>
    ipcRenderer.invoke('open-folder', folderPath),

  loadSettings: (): Promise<AppSettings> =>
    ipcRenderer.invoke('load-settings'),

  saveSettings: (settings: AppSettings): Promise<void> =>
    ipcRenderer.invoke('save-settings', settings),

  windowMinimize: (): Promise<void> =>
    ipcRenderer.invoke('window-minimize'),

  windowMaximize: (): Promise<void> =>
    ipcRenderer.invoke('window-maximize'),

  windowClose: (): Promise<void> =>
    ipcRenderer.invoke('window-close'),

  getPlatform: (): string => process.platform
})
