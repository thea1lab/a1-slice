import CenteredMessage from '../components/CenteredMessage'
import TranscribeProgress from '../components/TranscribeProgress'
import TranscribeSetup from '../components/TranscribeSetup'
import type { PipelineStage, VideoLanguage } from '../../shared/types'

interface TranscribeScreenProps {
  projectReady: boolean
  videoPath: string
  language: VideoLanguage
  entropyThold: number
  maxContext: number
  beamSize: number
  temperatureInc: number
  savedCount: number
  continueNote: string | null
  stage: PipelineStage
  message: string
  percent: number
  error: string | null
  onLanguageChange: (language: VideoLanguage) => void
  onEntropyTholdChange: (value: number) => void
  onMaxContextChange: (value: number) => void
  onBeamSizeChange: (value: number) => void
  onTemperatureIncChange: (value: number) => void
  onStart: () => void
  onUseSaved: () => void
  onCancel: () => void
}

export default function TranscribeScreen({
  projectReady,
  stage,
  message,
  percent,
  error,
  onStart,
  onCancel,
  ...setup
}: TranscribeScreenProps): React.JSX.Element {
  if (!projectReady && stage === 'idle') {
    return <CenteredMessage>Opening the video…</CenteredMessage>
  }
  if (stage === 'idle' || stage === 'done') {
    return <TranscribeSetup {...setup} onStart={onStart} />
  }
  return (
    <TranscribeProgress
      message={message}
      percent={percent}
      error={error}
      onCancel={onCancel}
      onRetry={onStart}
    />
  )
}
