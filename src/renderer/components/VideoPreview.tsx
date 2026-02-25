import { useRef, useEffect, useState } from 'react'
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
  const boundsRef = useRef({ startSec: startMs / 1000, endSec: endMs / 1000 })
  const [visible, setVisible] = useState(false)

  // Keep boundsRef in sync
  boundsRef.current = { startSec: startMs / 1000, endSec: endMs / 1000 }

  const src = `a1slice://video?path=${encodeURIComponent(videoPath)}`

  // Init / dispose player when src changes — all listeners registered here
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    setVisible(false)

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

    const seekToStart = (): void => {
      player.currentTime(boundsRef.current.startSec)
    }

    const onLoadedMetadata = (): void => {
      seekToStart()
    }

    const onLoadedData = (): void => {
      seekToStart()
    }

    const onSeeked = (): void => {
      setVisible(true)
    }

    const onTimeUpdate = (): void => {
      const current = player.currentTime()
      if (current !== undefined && current >= boundsRef.current.endSec) {
        player.currentTime(boundsRef.current.startSec)
      }
    }

    const onPlay = (): void => {
      const current = player.currentTime()
      const { startSec, endSec } = boundsRef.current
      if (current !== undefined && (current < startSec || current >= endSec)) {
        player.currentTime(startSec)
      }
    }

    player.on('loadedmetadata', onLoadedMetadata)
    player.on('loadeddata', onLoadedData)
    player.on('seeked', onSeeked)
    player.on('timeupdate', onTimeUpdate)
    player.on('play', onPlay)

    // If metadata already available (cached), seek immediately
    if (player.readyState() >= 1) {
      seekToStart()
    }

    return () => {
      if (playerRef.current) {
        playerRef.current.dispose()
        playerRef.current = null
      }
    }
  }, [src])

  // Re-seek when bounds change (player already exists)
  useEffect(() => {
    const player = playerRef.current
    if (!player) return

    if (player.readyState() >= 1) {
      setVisible(false)
      player.currentTime(startMs / 1000)
    }
  }, [startMs, endMs])

  return (
    <div className="w-full rounded-lg bg-black aspect-video overflow-hidden">
      <div
        ref={containerRef}
        data-vjs-player
        className="w-full h-full transition-opacity duration-150"
        style={{ opacity: visible ? 1 : 0 }}
      />
    </div>
  )
}
