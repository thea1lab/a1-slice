import ProgressBar from './ProgressBar'

interface ExportJobProps {
  running: boolean
  percent: number
  message: string
  error: string | null
  done: boolean
  actionLabel: string
  onAction: () => void
  onShow?: () => void
  resultPath?: string | null
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
  resultPath = null,
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
      <div className="flex items-center gap-4 min-w-0">
        {done && onShow ? (
          <button type="button" onClick={onShow} className="btn btn-primary shrink-0">
            Show file
          </button>
        ) : (
          <button type="button" onClick={onAction} disabled={running || disabled} className="btn btn-primary shrink-0">
            {actionLabel}
          </button>
        )}
        {resultPath && (
          <p className="min-w-0 text-[15px] leading-snug text-[#d9d3c5] break-all" title={resultPath}>
            {resultPath}
          </p>
        )}
      </div>
    </div>
  )
}
