import ProgressBar from './ProgressBar'
import type { PipelineStage } from '../../shared/types'

interface StepExportProps {
  stage: PipelineStage
  message: string
  percent: number
  outputDir: string | null
  error: string | null
  onOpenFolder: () => void
  onStartOver: () => void
  onCancel: () => void
}

export default function StepExport({
  stage,
  message,
  percent,
  outputDir,
  error,
  onOpenFolder,
  onStartOver,
  onCancel
}: StepExportProps): React.JSX.Element {
  const isDone = stage === 'done' && outputDir

  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-6 max-w-lg mx-auto w-full">
      {isDone ? (
        <div className="w-full bg-bg-card border border-emerald-800 rounded-2xl p-6 space-y-4 shadow-lg">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-emerald-400" />
            <span className="text-sm text-emerald-300 font-medium">
              Clips are ready
            </span>
          </div>
          <div className="flex gap-3">
            <button
              onClick={onOpenFolder}
              className="flex-1 bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg py-2.5 text-sm transition-colors"
            >
              Open Folder
            </button>
            <button
              onClick={onStartOver}
              className="flex-1 bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2.5 text-sm transition-colors"
            >
              Start Over
            </button>
          </div>
        </div>
      ) : error ? (
        <div className="w-full bg-bg-card border border-red-900 rounded-2xl p-6 space-y-4 shadow-lg">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-red-400" />
            <span className="text-sm text-red-300 font-medium">Error</span>
          </div>
          <p className="text-sm text-neutral-400">{error}</p>
          <button
            onClick={onStartOver}
            className="w-full bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2.5 text-sm transition-colors"
          >
            Start Over
          </button>
        </div>
      ) : (
        <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-6 space-y-5 shadow-lg">
          <h2 className="text-sm font-medium text-neutral-300">
            Exporting Clips
          </h2>
          <ProgressBar
            percent={percent}
            label="Cutting"
            sublabel={`${percent}%`}
          />
          <p className="text-xs text-neutral-500">{message}</p>
          <button
            onClick={onCancel}
            className="w-full bg-red-800 hover:bg-red-700 text-white font-medium rounded-lg py-2.5 text-sm transition-colors"
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  )
}
