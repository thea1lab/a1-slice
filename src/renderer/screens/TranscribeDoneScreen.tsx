import { useState } from 'react'
import TranscriptViewer from '../components/TranscriptViewer'
import { transcriptTextPath } from '../../shared/project'
import type { TranscriptSegment } from '../../shared/types'

interface TranscribeDoneProps {
  videoPath: string
  segments: TranscriptSegment[]
  onOpen: () => Promise<{ success: boolean; error?: string }>
  onHome: () => void
}

export default function TranscribeDone({
  videoPath,
  segments,
  onOpen,
  onHome
}: TranscribeDoneProps): React.JSX.Element {
  const [error, setError] = useState<string | null>(null)
  const [opening, setOpening] = useState(false)
  const lines = segments.length
  const filePath = videoPath ? transcriptTextPath(videoPath) : ''
  const lineLabel = lines === 1 ? '1 line is' : `${lines} lines are`

  const open = (): void => {
    setOpening(true)
    setError(null)
    void onOpen()
      .then((result) => {
        if (!result.success) setError(result.error || 'Could not open the transcript')
      })
      .catch(() => setError('Could not open the transcript'))
      .finally(() => setOpening(false))
  }

  return (
    <div className="sheet">
      <div>
        <h1 className="display">Transcript saved</h1>
        <p className="lead mt-3">
          {lineLabel} saved in a text file in the same folder as the video. The words are below.
        </p>
      </div>
      {filePath && (
        <div className="block">
          <p className="section-label">The file</p>
          <p className="file-path">{filePath}</p>
        </div>
      )}
      <div className="w-full">
        <TranscriptViewer segments={segments} />
      </div>
      <p className="lead">
        Go back to the home page to keep working on this video. From there you can find the best
        parts, reframe the picture, or add captions.
      </p>
      {error && <p className="text-base text-[#fa520f]">{error}</p>}
      <div className="actions">
        <button
          type="button"
          onClick={open}
          disabled={opening || !videoPath}
          className="btn btn-secondary btn-lg"
        >
          {opening ? 'Opening…' : 'Open transcription'}
        </button>
        <button type="button" onClick={onHome} className="btn btn-primary btn-lg">
          Back to home
        </button>
      </div>
    </div>
  )
}
