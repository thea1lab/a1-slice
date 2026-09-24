import type { SubtitleExport } from '../../shared/types'
import SegmentedChoice from './SegmentedChoice'

const MODES: { id: SubtitleExport; label: string }[] = [
  { id: 'off', label: 'Off' },
  { id: 'srt', label: 'Subtitle file' },
  { id: 'burn', label: 'On the video' }
]

interface SubtitleModePickerProps {
  value: SubtitleExport
  onChange: (mode: SubtitleExport) => void
}

export default function SubtitleModePicker({
  value,
  onChange
}: SubtitleModePickerProps): React.JSX.Element {
  return (
    <SegmentedChoice
      label="Subtitles on export"
      value={value}
      options={MODES}
      onChange={onChange}
      size="sm"
    />
  )
}