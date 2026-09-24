import ProgressBar from './ProgressBar'

interface ExportJobProps {
  running: boolean
  percent: number
  message: string
  error: string | null
  done: boolean
  actionLabel: string
  onAction: () => void
  onShow: () => void
  disabled?: boolean
}

export default function ExportJob({
  running,
  percent,
  message,
  error,
  done,
  actionLabel,
  onAction,
  onShow,
  disabled = false
}: ExportJobProps): React.JSX.Element {
  return (
    <div className="space-y-3">
      {running && (
        <ProgressBar
          percent={percent}
          label="Rendering"
          sublabel={message || `${percent}%`}
        />
      )}
      {error && <p className="text-sm text-red-300">{error}</p>}
      {done ? (
        <button type="button" onClick={onShow} className="btn btn-primary">
          Show file
        </button>
      ) : (
        <button type="button" onClick={onAction} disabled={running || disabled} className="btn btn-primary">
          {actionLabel}
        </button>
      )}
    </div>
  )
}
