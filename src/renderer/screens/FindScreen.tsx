import CenteredMessage from '../components/CenteredMessage'
import FindClipsForm from '../components/FindClipsForm'
import type { ClipSegment, LLMProvider, TranscriptSegment } from '../../shared/types'

interface FindScreenProps {
  projectReady: boolean
  segments: TranscriptSegment[]
  videoPath: string
  provider: LLMProvider
  model: string
  apiKey: string
  userHint: string
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  error: string | null
  hasClips: boolean
  onProviderChange: (provider: LLMProvider) => void
  onModelChange: (model: string) => void
  onApiKeyChange: (apiKey: string) => void
  onUserHintChange: (userHint: string) => void
  onAnalyze: (userHint?: string) => void
  onLoadCachedAnalysis: (clips: ClipSegment[], rawResponse: string) => void
  onCancel: () => void
  onAddRange: () => void
  onBackToClips: () => void
  onTranscribe: () => void
}

export default function FindScreen({
  projectReady,
  segments,
  onTranscribe,
  hasClips,
  onBackToClips,
  ...form
}: FindScreenProps): React.JSX.Element {
  if (!projectReady) {
    return <CenteredMessage>Opening the video…</CenteredMessage>
  }
  if (segments.length === 0) {
    return (
      <div className="sheet">
        <div>
          <h1 className="display">Find best parts</h1>
          <p className="lead mt-3">
            This video has no transcript yet. The words have to be written down before the best
            parts can be found.
          </p>
        </div>
        <button type="button" onClick={onTranscribe} className="btn btn-primary btn-lg">
          Transcribe this video
        </button>
      </div>
    )
  }
  return (
    <FindClipsForm
      {...form}
      segments={segments}
      onAddRange={form.onAddRange}
      onBackToClips={hasClips ? onBackToClips : undefined}
    />
  )
}
