import { useMemo, useState } from 'react'
import VideoPreview from '../components/VideoPreview'
import ExportJob from '../components/ExportJob'
import ChoiceCard from '../components/ChoiceCard'
import CaptionOverlay from '../components/CaptionOverlay'
import type { CaptionLook, CaptionProject, CaptionSource, PipelineStage, TranscriptSegment } from '../../shared/types'
import { parseSrt } from '../../shared/project'

const LOOKS: { id: CaptionLook; title: string; detail: string }[] = [
  { id: 'srt', title: 'Subtitle file', detail: 'An .srt you can edit later. The picture stays as it is.' },
  { id: 'burn-large', title: 'On the video, large', detail: 'Big type at the bottom, for stories and reels.' },
  { id: 'burn-small', title: 'On the video, small', detail: 'Smaller type, for a YouTube frame.' }
]

interface CaptionsScreenProps {
  videoPath: string
  durationMs: number
  segments: TranscriptSegment[]
  captions: CaptionProject | null
  exportStage: PipelineStage
  exportMessage: string
  exportPercent: number
  outputDir: string | null
  exportError: string | null
  onChange: (captions: CaptionProject) => void
  onTranscribe: () => void
  onExport: (cues: TranscriptSegment[], look: CaptionLook) => void
  onOpenFolder: () => void
}

export default function CaptionsScreen({
  videoPath,
  durationMs,
  segments,
  captions,
  exportStage,
  exportMessage,
  exportPercent,
  outputDir,
  exportError,
  onChange,
  onTranscribe,
  onExport,
  onOpenFolder
}: CaptionsScreenProps): React.JSX.Element {
  const project: CaptionProject = captions ?? {
    source: segments.length > 0 ? 'transcript' : 'manual',
    look: 'srt',
    cues: []
  }
  const [playMs, setPlayMs] = useState(0)
  const [paste, setPaste] = useState('')
  const [pasteError, setPasteError] = useState<string | null>(null)
  const cues = project.source === 'transcript' ? segments : project.cues
  const active = cues.find((cue) => playMs >= cue.startMs && playMs <= cue.endMs)
  const endMs = durationMs > 0 ? durationMs : Math.max(1000, cues.at(-1)?.endMs ?? 1000)
  const rendering = exportStage === 'cutting'
  const done = exportStage === 'done' && outputDir

  const sample = useMemo(() => active?.text ?? cues[0]?.text ?? 'Caption preview', [active, cues])

  const setSource = (source: CaptionSource): void => {
    onChange({ ...project, source })
  }

  const applyPaste = (): void => {
    const parsed = parseSrt(paste)
    if (parsed.length === 0) {
      setPasteError('That was not a subtitle file. Keep the time lines like 00:00:01,000 --> 00:00:03,000.')
      return
    }
    setPasteError(null)
    onChange({ ...project, source: 'manual', cues: parsed })
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 w-full">
      <div className="relative flex-1 min-h-0 bg-black">
        <VideoPreview
          videoPath={videoPath}
          startMs={0}
          endMs={endMs}
          videoDurationMs={durationMs || undefined}
          editor
          onPlayheadMs={setPlayMs}
        />
        <CaptionOverlay text={sample} look={project.look} />
      </div>

      <div className="dock overflow-y-auto max-h-[52%] custom-scrollbar">
        <div className="block">
          <h2 className="section-label">Where do the words come from?</h2>
          <p className="help">
            Use a transcript already saved with this video, write one now, or paste subtitles.
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 w-full">
            <ChoiceCard
              title="Saved transcript"
              detail={segments.length > 0 ? `${segments.length} lines` : 'None yet'}
              selected={project.source === 'transcript'}
              disabled={segments.length === 0}
              onClick={() => setSource('transcript')}
            />
            <ChoiceCard
              title="Transcribe now"
              detail="Write the words, then come back here."
              onClick={onTranscribe}
            />
            <ChoiceCard
              title="Type or paste"
              detail={project.cues.length > 0 ? `${project.cues.length} lines` : 'Paste a subtitle file'}
              selected={project.source === 'manual'}
              onClick={() => setSource('manual')}
            />
          </div>
        </div>

        {project.source === 'manual' && (
          <div className="space-y-2">
            <textarea
              value={paste}
              onChange={(event) => setPaste(event.target.value)}
              rows={4}
              placeholder={'1\n00:00:01,000 --> 00:00:03,000\nHello'}
              className="field font-mono text-[15px]"
            />
            <button type="button" onClick={applyPaste} className="btn btn-secondary">
              Use this subtitle file
            </button>
            {pasteError && <p className="text-base text-red-300">{pasteError}</p>}
          </div>
        )}

        <div className="block">
          <h2 className="section-label">How should they look?</h2>
          <p className="help">Save a subtitle file, or write the words onto the picture.</p>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-2 w-full">
            {LOOKS.map((look) => (
              <ChoiceCard
                key={look.id}
                title={look.title}
                detail={look.detail}
                selected={project.look === look.id}
                onClick={() => onChange({ ...project, look: look.id })}
              />
            ))}
          </div>
        </div>

        <ExportJob
          running={rendering}
          percent={exportPercent}
          message={exportMessage}
          error={exportError}
          done={Boolean(done)}
          actionLabel="Export captions"
          disabled={cues.length === 0}
          onAction={() => onExport(cues, project.look)}
          onShow={onOpenFolder}
        />
      </div>
    </div>
  )
}
