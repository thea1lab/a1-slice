import { useRef, useEffect } from 'react'
import videojs from 'video.js'
import Player from 'video.js/dist/types/player'
import 'video.js/dist/video-js.css'

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
  const containerRef = useRef<HTMLDivElement>(null)
  const playerRef = useRef<Player | null>(null)

  const src = `a1slice://video?path=${encodeURIComponent(videoPath)}`

  // Init / dispose player when src changes
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    // Create <video-js> element inside the container
    const videoEl = document.createElement('video-js')
    videoEl.classList.add('vjs-big-play-centered')
    container.appendChild(videoEl)

    const player = videojs(videoEl, {
      controls: true,
      muted: false,
      preload: 'auto',
      sources: [{ src, type: 'video/mp4' }]
    })

    playerRef.current = player

    return () => {
      if (playerRef.current) {
        playerRef.current.dispose()
        playerRef.current = null
      }
    }
  }, [src])

  // Time-boundary effect: seek to start, pause at end
  useEffect(() => {
    const player = playerRef.current
    if (!player) return

    const startSec = startMs / 1000
    const endSec = endMs / 1000

    const seekToStart = (): void => {
      player.currentTime(startSec)
    }

    const onLoadedData = (): void => {
      seekToStart()
    }

    const onTimeUpdate = (): void => {
      const current = player.currentTime()
      if (current !== undefined && current >= endSec) {
        player.currentTime(startSec)
      }
    }

    const onPlay = (): void => {
      const current = player.currentTime()
      if (current !== undefined && (current < startSec || current >= endSec)) {
        player.currentTime(startSec)
      }
    }

    player.on('loadeddata', onLoadedData)
    player.on('timeupdate', onTimeUpdate)
    player.on('play', onPlay)

    // If video data is already loaded (e.g. startMs/endMs changed after load), seek now
    if (player.readyState() >= 2) {
      seekToStart()
    }

    return () => {
      player.off('loadeddata', onLoadedData)
      player.off('timeupdate', onTimeUpdate)
      player.off('play', onPlay)
    }
  }, [startMs, endMs])

  return (
    <div
      ref={containerRef}
      data-vjs-player
      className="w-full rounded-lg bg-black aspect-video overflow-hidden"
    />
  )
}
