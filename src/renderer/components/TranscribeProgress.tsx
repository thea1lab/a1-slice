import ProgressBar from './ProgressBar'

interface TranscribeProgressProps {
  message: string
  percent: number
  error: string | null
  onCancel: () => void
  onRetry: () => void
}

export default function TranscribeProgress({
  message,
  percent,
  error,
  onCancel,
  onRetry
}: TranscribeProgressProps): React.JSX.Element {
  return (
    <div className="sheet">
      <div>
        <h1 className="display">Transcribing</h1>
        <p className="lead mt-3">
          The speech is being written down on this computer. Stay here until it finishes.
        </p>
      </div>

      {error ? (
        <div className="block">
          {error === 'Cancelled' ? (
            <p className="help">Cancelled. You can start again.</p>
          ) : (
            <p className="text-base text-red-300">{error}</p>
          )}
          <button type="button" onClick={onRetry} className="btn btn-primary btn-lg">
            Try again
          </button>
        </div>
      ) : (
        <div className="block">
          <ProgressBar percent={percent} label={message || 'Working'} sublabel={`${percent}%`} />
          <p className="help">
            If it sits still for a long time, cancel. Then open If the words come out wrong, raise
            Drop uncertain words, or set Words it remembers to 0, and start again.
          </p>
          <button type="button" onClick={onCancel} className="btn btn-secondary">
            Cancel
          </button>
        </div>
      )}
    </div>
  )
}
