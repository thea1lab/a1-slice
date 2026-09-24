import ProgressBar from '../components/ProgressBar'
import type { PipelineStage } from '../../shared/types'

interface ExportScreenProps {
  stage: PipelineStage
  message: string
  percent: number
  outputDir: string | null
  error: string | null
  onOpenFolder: () => void
  onStartOver: () => void
  onCancel: () => void
  onReframe?: () => void
  onCaptions?: () => void
}

export default function ExportScreen({
  stage,
  message,
  percent,
  outputDir,
  error,
  onOpenFolder,
  onStartOver,
  onCancel,
  onReframe,
  onCaptions
}: ExportScreenProps): React.JSX.Element {
  const isDone = stage === 'done' && outputDir
  const folderName = outputDir ? outputDir.split(/[/\\]/).pop() : ''

  return (
    <div className="sheet">
      <h1 className="display">
        {isDone ? 'Export complete' : error ? 'Export failed' : 'Exporting'}
      </h1>
      {isDone ? (
        <div className="block">
          <p className="lead">
            The clips are saved next to the original video, in {folderName}.
          </p>
          <div className="actions">
            <button type="button" onClick={onOpenFolder} className="btn btn-primary btn-lg">
              Open folder
            </button>
            {onReframe && (
              <button type="button" onClick={onReframe} className="btn btn-secondary btn-lg">
                Reframe
              </button>
            )}
            {onCaptions && (
              <button type="button" onClick={onCaptions} className="btn btn-secondary btn-lg">
                Captions
              </button>
            )}
            <button type="button" onClick={onStartOver} className="btn btn-secondary btn-lg">
              Back to home
            </button>
          </div>
        </div>
      ) : error ? (
        <div className="block">
          <p className="text-base text-[#fa520f]">{error}</p>
          <button type="button" onClick={onStartOver} className="btn btn-secondary btn-lg">
            Back to home
          </button>
        </div>
      ) : (
        <div className="block">
          <ProgressBar percent={percent} label="Exporting the clips" sublabel={`${percent}%`} />
          {message && <p className="help">{message}</p>}
          <button type="button" onClick={onCancel} className="btn btn-danger">
            Cancel
          </button>
        </div>
      )}
    </div>
  )
}