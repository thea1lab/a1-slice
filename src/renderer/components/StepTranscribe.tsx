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
    <div className="flex flex-col justify-center flex-1 gap-5 max-w-lg mx-auto w-full">
      <div>
        <h2 className="text-lg font-medium text-neutral-100">Transcribing</h2>
        <p className="mt-1 text-sm text-neutral-500">
          Turning the audio into text. Stay on this screen until it finishes.
        </p>
      </div>

      {error ? (
        <div className="space-y-4">
          {error === 'Cancelled' ? (
            <p className="text-sm text-neutral-400">Cancelled.</p>
          ) : (
            <p className="text-sm text-red-300">{error}</p>
          )}
          <button
            onClick={onRetry}
            className="w-full bg-bg-input border border-white/12 hover:border-white/25 text-neutral-200 font-medium rounded-lg py-2 text-sm transition-colors"
          >
            Try again
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          <ProgressBar
            percent={percent}
            label={message || 'Working'}
            sublabel={`${percent}%`}
          />
          <p className="text-[11px] text-neutral-600">
            If this stalls, cancel and raise entropy or lower max context under Advanced on Select.
          </p>
          <button
            onClick={onCancel}
            className="text-sm text-white/50 hover:text-white"
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  )
}
