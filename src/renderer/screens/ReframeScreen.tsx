import { useState } from 'react'
import VideoPreview from '../components/VideoPreview'
import ExportJob from '../components/ExportJob'
import ChoiceCard from '../components/ChoiceCard'
import type { ClipCrop, CropRatio, PipelineStage } from '../../shared/types'
import { DEFAULT_CROP } from '../../shared/types'
import { CROP_PRESETS } from '../../shared/crop'

interface ReframeScreenProps {
  videoPath: string
  durationMs: number
  framing: Partial<Record<CropRatio, ClipCrop>>
  exportStage: PipelineStage
  exportMessage: string
  exportPercent: number
  outputDir: string | null
  exportError: string | null
  onFramingChange: (crop: ClipCrop) => void
  onExport: (crop: ClipCrop) => void
  onOpenFolder: () => void
}

export default function ReframeScreen({
  videoPath,
  durationMs,
  framing,
  exportStage,
  exportMessage,
  exportPercent,
  outputDir,
  exportError,
  onFramingChange,
  onExport,
  onOpenFolder
}: ReframeScreenProps): React.JSX.Element {
  const [ratio, setRatio] = useState<CropRatio>(() => {
    const saved = (['9:16', '16:9', '1:1', '4:3'] as CropRatio[]).find((key) => framing[key])
    return saved ?? '9:16'
  })
  const crop: ClipCrop =
    ratio === 'original' ? { ...DEFAULT_CROP } : framing[ratio] ?? { ratio, cx: 0.5, cy: 0.5, zoom: 0 }
  const endMs = durationMs > 0 ? durationMs : 1000
  const rendering = exportStage === 'cutting'
  const done = exportStage === 'done' && outputDir

  const choose = (next: CropRatio): void => {
    setRatio(next)
    if (next === 'original') return
    if (!framing[next]) onFramingChange({ ratio: next, cx: 0.5, cy: 0.5, zoom: 0 })
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 w-full bg-black">
      <div className="relative flex-1 min-h-0">
        <VideoPreview
          videoPath={videoPath}
          startMs={0}
          endMs={endMs}
          videoDurationMs={durationMs || undefined}
          crop={crop}
          editor
          onCropChange={ratio === 'original' ? undefined : onFramingChange}
        />
      </div>
      <div className="dock">
        <div className="block">
          <h2 className="section-label">Shape of the picture</h2>
          <p className="help">
            Drag the frame until the subject sits inside it. Press space to play and pause, or
            use the play button under the picture. Scroll to zoom in. This frame is saved on the
            video, and export writes one new file.
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {CROP_PRESETS.map((preset) => (
            <ChoiceCard
              key={preset.ratio}
              title={preset.label}
              detail={preset.detail}
              selected={ratio === preset.ratio}
              onClick={() => choose(preset.ratio)}
            />
          ))}
        </div>
        <ExportJob
          running={rendering}
          percent={exportPercent}
          message={exportMessage}
          error={exportError}
          done={Boolean(done)}
          actionLabel="Export this frame"
          onAction={() => onExport(crop)}
          onShow={onOpenFolder}
        />
      </div>
    </div>
  )
}
