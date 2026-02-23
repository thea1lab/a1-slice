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

  const src = `a1slice://video?path=${encodeURIComponent(videoPath)}`

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

    const seekToStart = (): void => {
      video.currentTime = startMs / 1000
    }

    if (video.readyState >= 1) {
      seekToStart()
    } else {
      video.addEventListener('loadedmetadata', seekToStart, { once: true })
      return () => video.removeEventListener('loadedmetadata', seekToStart)
    }
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
