import { useState } from 'react'
import VideoPreview from '../components/VideoPreview'
import ExportJob from '../components/ExportJob'
import CaptionOverlay from '../components/CaptionOverlay'
import CaptionStylePicker from '../components/CaptionStylePicker'
import type { CaptionProject, CaptionStyle, PipelineStage, TranscriptSegment } from '../../shared/types'
import { presentCaptionProject } from '../../shared/captions'
import { parseCaptionDocument, transcriptTextPath } from '../../shared/project'

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
  onExport: (cues: TranscriptSegment[], look: 'burn', style: CaptionStyle) => void
  onFixWords: () => void
}

function sameFile(left: string, right: string): boolean {
  const norm = (value: string): string => value.replace(/\\/g, '/').replace(/\/+$/, '')
  return norm(left) === norm(right)
}

function remembered(project: CaptionProject, patch: Partial<CaptionProject>): CaptionProject {
  const next: CaptionProject = { ...project, ...patch, look: 'burn' }
  if (next.source !== 'manual') delete next.filePath
  return next
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
  onExport,
  onFixWords
}: CaptionsScreenProps): React.JSX.Element {
  const project: CaptionProject = presentCaptionProject(captions, segments.length > 0)
  const [playMs, setPlayMs] = useState(0)
  const [fileError, setFileError] = useState<string | null>(null)
  const cues = project.source === 'transcript' ? segments : project.cues
  const active = cues.find((cue) => playMs >= cue.startMs && playMs <= cue.endMs)
  const endMs = durationMs > 0 ? durationMs : Math.max(1000, cues.at(-1)?.endMs ?? 1000)
  const rendering = exportStage === 'cutting'
  const done = exportStage === 'done' && outputDir
  const sample = active?.text ?? cues[0]?.text ?? ''
  const transcriptPath = transcriptTextPath(videoPath)
  const loadedPath = project.source === 'transcript'
    ? (segments.length > 0 ? transcriptPath : '')
    : (project.filePath ?? '')
  const loadedLabel = loadedPath || (project.cues.length > 0 ? 'Pasted subtitles' : 'No caption file yet')

  const chooseFile = (): void => {
    void window.api.selectCaptionFile().then((picked) => {
      if (!picked) return
      if (picked.error || !picked.text) {
        setFileError(picked.error || 'That file could not be read.')
        return
      }
      const parsed = parseCaptionDocument(picked.text)
      if (parsed.length === 0) {
        setFileError('That file has no captions. Use a subtitle file or a transcript.')
        return
      }
      setFileError(null)
      if (segments.length > 0 && sameFile(picked.path, transcriptPath)) {
        onChange(remembered(project, { source: 'transcript', cues: [] }))
        return
      }
      onChange(remembered(project, { source: 'manual', cues: parsed, filePath: picked.path }))
    }).catch(() => {
      setFileError('That file could not be read.')
    })
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
        >
          <CaptionOverlay text={sample} style={project.style} />
        </VideoPreview>
      </div>

      <div className="dock overflow-y-auto max-h-[68%] custom-scrollbar">
        <CaptionStylePicker
          style={project.style}
          onChange={(style) => onChange(remembered(project, { style }))}
        />

        <div className="block">
          <h2 className="section-label">Where do the words come from?</h2>
          <div className="caption-file">
            <p className="caption-file-name" title={loadedPath || undefined}>
              {loadedLabel}
            </p>
            <div className="caption-file-actions">
              <button
                type="button"
                className="icon-button"
                aria-label="Choose another caption file"
                title="Choose another caption file"
                onClick={chooseFile}
              >
                <FolderIcon />
              </button>
              <button
                type="button"
                className="icon-button"
                aria-label="Edit these captions"
                title="Edit these captions"
                disabled={cues.length === 0}
                onClick={onFixWords}
              >
                <PencilIcon />
              </button>
            </div>
          </div>
          {cues.length === 0 && (
            <p className="help">
              If you don't have the captions yet, open Transcribe on the home page.
            </p>
          )}
          {fileError && <p className="text-base text-red-300">{fileError}</p>}
        </div>

        <ExportJob
          running={rendering}
          percent={exportPercent}
          message={exportMessage}
          error={exportError}
          done={false}
          actionLabel="Export captions"
          disabled={cues.length === 0}
          resultPath={done && outputDir ? outputDir : null}
          onAction={() => onExport(cues, 'burn', project.style)}
        />
      </div>
    </div>
  )
}

function FolderIcon(): React.JSX.Element {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M2 4.5h4l1.2 1.5H14V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4.5Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
    </svg>
  )
}

function PencilIcon(): React.JSX.Element {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M9.2 3.2 12.8 6.8 5.5 14.1 2 14.9 2.8 11.4 9.2 3.2Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
      <path d="M8.2 4.2 11.8 7.8" fill="none" stroke="currentColor" strokeWidth="1.4" />
    </svg>
  )
}
