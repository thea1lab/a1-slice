import ProgressBar from './ProgressBar'

interface StepTranscribeProps {
  message: string
  percent: number
  error: string | null
  onCancel: () => void
  onRetry: () => void
}

export default function StepTranscribe({
  message,
  percent,
  error,
  onCancel,
  onRetry
}: StepTranscribeProps): React.JSX.Element {
  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-6 max-w-lg mx-auto w-full">
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-6 space-y-5 shadow-lg">
        <h2 className="text-sm font-medium text-neutral-300">Transcribing Video</h2>

        {error ? (
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-red-400" />
              <span className="text-sm text-red-300 font-medium">Error</span>
            </div>
            <p className="text-sm text-neutral-400">{error}</p>
            <button
              onClick={onRetry}
              className="w-full bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2 text-sm transition-colors"
            >
              Try Again
            </button>
          </div>
        ) : (
          <div className="space-y-4">
            <ProgressBar
              percent={percent}
              label={message}
              sublabel={`${percent}%`}
            />
            <button
              onClick={onCancel}
              className="w-full bg-red-800 hover:bg-red-700 text-white font-medium rounded-lg py-2.5 text-sm transition-colors"
            >
              Cancel
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
