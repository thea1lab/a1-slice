import { useState, useEffect } from 'react'
import type { TranscriptSegment, VideoLanguage } from '../../shared/types'

interface StepSelectVideoProps {
  videoPath: string | null
  language: VideoLanguage
  onLanguageChange: (language: VideoLanguage) => void
  onSelectVideo: () => void
  onNext: () => void
  onLoadCachedTranscript?: (segments: TranscriptSegment[]) => void
}

export default function StepSelectVideo({
  videoPath,
  language,
  onLanguageChange,
  onSelectVideo,
  onNext,
  onLoadCachedTranscript
}: StepSelectVideoProps): React.JSX.Element {
  const [cachedSegments, setCachedSegments] = useState<TranscriptSegment[] | null>(null)
  const [checking, setChecking] = useState(false)

  useEffect(() => {
    if (!videoPath) {
      setCachedSegments(null)
      return
    }
    setChecking(true)
    window.api
      .checkTranscript(videoPath)
      .then((result) => {
        setCachedSegments(result.found && result.segments ? result.segments : null)
      })
      .catch(() => setCachedSegments(null))
      .finally(() => setChecking(false))
  }, [videoPath])

  return (
    <div className="flex flex-col items-center justify-center flex-1 gap-6 max-w-lg mx-auto w-full">
      {/* Header */}
      <div className="flex items-center gap-3">
        <svg
          viewBox="0 0 32 32"
          fill="none"
          className="w-9 h-9 shrink-0"
          aria-hidden="true"
        >
          <rect x="2" y="6" width="28" height="20" rx="4" stroke="rgb(240,154,62)" strokeWidth="1.5" opacity="0.5" />
          <line x1="12" y1="6" x2="12" y2="26" stroke="rgb(240,154,62)" strokeWidth="1.5" strokeDasharray="3 2" />
          <polygon points="10,14 14,16 10,18" fill="rgb(240,154,62)" opacity="0.8" />
          <rect x="4" y="10" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
          <rect x="4" y="15" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
          <rect x="4" y="20" width="5" height="3" rx="0.5" fill="rgb(240,154,62)" opacity="0.3" />
          <rect x="16" y="12" width="11" height="2" rx="1" fill="white" opacity="0.15" />
          <rect x="16" y="17" width="8" height="2" rx="1" fill="white" opacity="0.10" />
        </svg>
        <div>
          <h1 className="text-2xl font-bold text-white tracking-tight">
            A1 <span style={{ color: 'rgb(240, 154, 62)' }}>Slice</span>
          </h1>
          <p className="text-xs text-neutral-500">Cut long videos into short reels</p>
        </div>
      </div>

      {/* Video Card */}
      <div className="w-full bg-bg-card border border-white/7 rounded-2xl p-6 space-y-4 shadow-lg">
        <h2 className="text-sm font-medium text-neutral-300">Select a video file</h2>
        <div className="flex items-center gap-3">
          <button
            onClick={onSelectVideo}
            className="bg-bg-input border border-white/12 rounded-lg px-4 py-2.5 text-sm text-neutral-200 hover:border-white/25 transition-colors shrink-0"
          >
            Choose File
          </button>
          {videoPath && (
            <span className="text-sm text-neutral-400 truncate">
              {videoPath.split(/[\\/]/).pop()}
            </span>
          )}
        </div>

        {/* Language selector */}
        {videoPath && (
          <label className="flex flex-col gap-1.5 text-xs text-neutral-400">
            Video language
            <select
              value={language}
              onChange={(e) => onLanguageChange(e.target.value as VideoLanguage)}
              className="appearance-none bg-bg-input border border-white/12 rounded-lg pl-3 pr-8 py-2 text-sm text-neutral-200 outline-none focus:border-accent transition-colors select-chevron w-48"
            >
              <option value="auto">Auto-detect</option>
              <option value="en">English</option>
              <option value="pt">Portuguese</option>
              <option value="es">Spanish</option>
            </select>
          </label>
        )}

        {/* Cached transcript prompt */}
        {videoPath && !checking && cachedSegments && onLoadCachedTranscript && (
          <div className="bg-bg-input border border-accent/20 rounded-lg p-4 space-y-3">
            <p className="text-sm text-neutral-300">
              Found an existing transcript for this video ({cachedSegments.length} segments).
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => onLoadCachedTranscript(cachedSegments)}
                className="bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg px-4 py-2 text-sm transition-colors"
              >
                Use Existing Transcript
              </button>
              <button
                onClick={onNext}
                className="bg-bg-input border border-white/12 rounded-lg px-4 py-2 text-sm text-neutral-200 hover:border-white/25 transition-colors"
              >
                Re-transcribe
              </button>
            </div>
          </div>
        )}

        {/* Normal next button (hidden when cached transcript is available) */}
        {(!cachedSegments || !onLoadCachedTranscript) && (
          <button
            onClick={onNext}
            disabled={!videoPath}
            className="w-full bg-accent hover:bg-accent-hover text-black font-semibold rounded-lg py-2.5 text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed disabled:hover:bg-accent"
          >
            Next
          </button>
        )}
      </div>
    </div>
  )
}
