import { useEffect, useState } from 'react'
import CenteredMessage from '../components/CenteredMessage'
import FindClipsForm from '../components/FindClipsForm'
import type { ClipSegment, TranscriptSegment } from '../../shared/types'

interface ClipAgent {
  id: string
  label: string
}

interface FindScreenProps {
  projectReady: boolean
  segments: TranscriptSegment[]
  videoPath: string
  userHint: string
  analyzing: boolean
  analyzePercent: number
  analyzeMessage: string
  error: string | null
  hasClips: boolean
  onUserHintChange: (userHint: string) => void
  onAnalyze: (agentId: string, userHint?: string) => void
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
  const [agents, setAgents] = useState<ClipAgent[] | null>(null)
  const [agentId, setAgentId] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    window.api
      .listCaptionAgents()
      .then((found) => {
        if (!active) return
        setAgents(found)
        setAgentId((current) =>
          current && found.some((agent) => agent.id === current) ? current : (found[0]?.id ?? null)
        )
      })
      .catch(() => {
        if (active) setAgents([])
      })
    return () => {
      active = false
    }
  }, [])

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
      agents={agents}
      agentId={agentId}
      onAgentChange={setAgentId}
      onAddRange={form.onAddRange}
      onBackToClips={hasClips ? onBackToClips : undefined}
    />
  )
}
