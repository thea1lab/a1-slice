import { useEffect, useState } from 'react'
import type { RecentVideo, ToolId } from '../../shared/types'
import ToolIcon from './ToolIcon'
import { fileNameOf } from '../lib/fileName'

const GUIDES: Record<ToolId, { title: string; lead: string; steps: string[] }> = {
  transcribe: {
    title: 'Transcribe',
    lead: 'Turn the speech in a video into a transcript. It stays on this computer, in a file next to the video.',
    steps: [
      'Pick the video.',
      'Tell us which language is spoken, or leave auto-detect on if you are not sure.',
      'Start. The transcript is saved next to the video, and you can leave after that.'
    ]
  },
  find: {
    title: 'Find best parts',
    lead: 'Mark the moments worth keeping, fix where each one starts and ends, then export those clips.',
    steps: [
      'Pick the video.',
      'If it has no transcript yet, the words are written down first so they can be searched.',
      'Keep or drop each part, nudge the start and end, then export.'
    ]
  },
  reframe: {
    title: 'Reframe',
    lead: 'Crop the picture for a phone story, YouTube, a square, or 4:3. The frame is remembered for this video.',
    steps: [
      'Pick the video.',
      'Choose a shape, then drag the frame until the subject sits inside it. Scroll to zoom.',
      'Export one new file.'
    ]
  },
  captions: {
    title: 'Captions',
    lead: 'Put the words on the picture. You can see the color, size, and font on the video before you export.',
    steps: [
      'Pick the video.',
      'The transcript saved with the video is loaded. Pick another caption file, or edit the words.',
      'Set the color, position, size, and font, then export.'
    ]
  }
}

function savedNote(video: RecentVideo): string | null {
  const parts: string[] = []
  if (video.hasTranscript) parts.push('Transcript saved')
  if (video.hasClips) parts.push(video.clipCount === 1 ? '1 clip' : `${video.clipCount} clips`)
  if (video.hasFraming) parts.push('Frame saved')
  if (video.hasCaptions) parts.push('Captions saved')
  return parts.length > 0 ? parts.join(' · ') : null
}

interface ToolStartProps {
  tool: ToolId
  onChoose: () => void
  onUseRecent: (path: string) => void
}

export default function ToolStart({ tool, onChoose, onUseRecent }: ToolStartProps): React.JSX.Element {
  const guide = GUIDES[tool]
  const [recent, setRecent] = useState<RecentVideo[]>([])

  useEffect(() => {
    window.api
      .listRecent()
      .then((videos) => setRecent(videos))
      .catch(() => setRecent([]))
  }, [])

  return (
    <div className="sheet">
      <span className="icon-tile icon-tile-lg">
        <ToolIcon id={tool} />
      </span>
      <div>
        <h1 className="display">{guide.title}</h1>
        <p className="lead mt-3">{guide.lead}</p>
      </div>
      <div className="block">
        <h2 className="section-label">What you do</h2>
        <ol className="steps">
          {guide.steps.map((step, index) => (
            <li key={step}>
              <span className="step-index">{String(index + 1).padStart(2, '0')}</span>
              {step}
            </li>
          ))}
        </ol>
      </div>
      <div className="flex flex-wrap items-center gap-4">
        <button type="button" className="btn btn-primary btn-lg" onClick={onChoose}>
          Choose a video
        </button>
        <p className="formats">MP4, MOV, MKV, AVI, or WebM</p>
      </div>
      {recent.length > 0 && (
        <section className="recent">
          <h2 className="section-label">Recent videos</h2>
          <p className="help">
            Open a file you used before. To work on a different video, choose one above.
          </p>
          <ul className="recent-list">
            {recent.map((video) => {
              const note = savedNote(video)
              const name = fileNameOf(video.path)
              return (
                <li key={video.path}>
                  <button type="button" onClick={() => onUseRecent(video.path)}>
                    <span className="recent-name">{name}</span>
                    {note ? <span className="recent-note">{note}</span> : null}
                  </button>
                </li>
              )
            })}
          </ul>
        </section>
      )}
    </div>
  )
}
