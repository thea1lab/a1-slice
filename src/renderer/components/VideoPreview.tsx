import { useRef, useCallback, useEffect } from 'react'

interface VideoPreviewProps {
  videoPath: string
  startMs: number
  endMs: number
}

export default function VideoPreview({
  videoPath,
  startMs,
  endMs
}: VideoPreviewProps): React.JSX.Element {
  const videoRef = useRef<HTMLVideoElement>(null)

  const startSec = startMs / 1000
  const endSec = endMs / 1000
  const src = `a1slice://video?path=${encodeURIComponent(videoPath)}#t=${startSec},${endSec}`

  const handleTimeUpdate = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    if (video.currentTime >= endMs / 1000) {
      video.pause()
      video.currentTime = startMs / 1000
    }
  }, [startMs, endMs])

  const handlePlay = useCallback(() => {
    const video = videoRef.current
    if (!video) return
    if (video.currentTime < startMs / 1000 || video.currentTime >= endMs / 1000) {
      video.currentTime = startMs / 1000
    }
  }, [startMs, endMs])

  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    video.currentTime = startMs / 1000
  }, [startMs])

  return (
    <video
      ref={videoRef}
      src={src}
      onTimeUpdate={handleTimeUpdate}
      onPlay={handlePlay}
      controls
      muted
      preload="metadata"
      className="w-full rounded-lg bg-black aspect-video"
    />
  )
}
