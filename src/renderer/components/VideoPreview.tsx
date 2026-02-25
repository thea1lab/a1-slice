import React, { useRef, useEffect, useState } from 'react'
import videojs from 'video.js'
import Player from 'video.js/dist/types/player'
import 'video.js/dist/video-js.css'

interface VideoPreviewProps {
  videoPath: string
  startMs: number
  endMs: number
}

function sourceTypeForPath(path: string): string {
  const lower = path.toLowerCase()
  if (lower.endsWith('.mov')) return 'video/quicktime'
  if (lower.endsWith('.mkv')) return 'video/x-matroska'
  if (lower.endsWith('.avi')) return 'video/x-msvideo'
  if (lower.endsWith('.webm')) return 'video/webm'
  return 'video/mp4'
}

const VideoPreview = React.memo(function VideoPreview({
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
  const sourceType = sourceTypeForPath(videoPath)

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
      preload: 'metadata',
      sources: [{ src, type: sourceType }]
    })

    playerRef.current = player
    const revealTimer = window.setTimeout(() => setVisible(true), 1500)

    let seeking = false

    const seekToStart = (): void => {
      player.currentTime(boundsRef.current.startSec)
    }

    const onLoadedMetadata = (): void => {
      seekToStart()
      setVisible(true)
    }

    const onCanPlay = (): void => {
      seekToStart()
      setVisible(true)
    }

    const onSeeked = (): void => {
      seeking = false
      setVisible(true)
    }

    const loopToStart = (): void => {
      player.currentTime(boundsRef.current.startSec)
      player.play()
    }

    const onTimeUpdate = (): void => {
      if (seeking) return
      const current = player.currentTime()
      if (current !== undefined && current >= boundsRef.current.endSec) {
        loopToStart()
      }
    }

    const onEnded = (): void => {
      loopToStart()
    }

    const onPlay = (): void => {
      if (seeking) return
      const current = player.currentTime()
      const { startSec, endSec } = boundsRef.current
      if (current !== undefined && (current < startSec || current >= endSec)) {
        seeking = true
        player.currentTime(startSec)
      }
    }

    player.one('loadedmetadata', onLoadedMetadata)
    player.one('canplay', onCanPlay)
    player.on('seeked', onSeeked)
    player.on('timeupdate', onTimeUpdate)
    player.on('play', onPlay)
    player.on('ended', onEnded)
    player.on('error', () => setVisible(true))

    // Preserve scroll position across fullscreen toggle.
    // The browser resets scroll BEFORE fullscreenchange fires, so we capture
    // the scroll position on pointerdown (which always precedes fullscreen).
    const scrollParent = container.closest('.overflow-y-auto') as HTMLElement | null
    let savedScrollTop = scrollParent?.scrollTop ?? 0
    const onPointerDown = (): void => {
      savedScrollTop = scrollParent?.scrollTop ?? 0
    }
    container.addEventListener('pointerdown', onPointerDown, true)

    const onFullscreenChange = (): void => {
      if (!player.isFullscreen()) {
        requestAnimationFrame(() => {
          if (scrollParent) {
            scrollParent.scrollTop = savedScrollTop
          }
        })
      }
    }
    player.on('fullscreenchange', onFullscreenChange)

    // If metadata already available (cached), seek immediately
    if (player.readyState() >= 1) {
      seekToStart()
      setVisible(true)
    }

    return () => {
      window.clearTimeout(revealTimer)
      container.removeEventListener('pointerdown', onPointerDown, true)
      if (playerRef.current) {
        playerRef.current.dispose()
        playerRef.current = null
      }
    }
  }, [src, sourceType])

  // Re-seek when bounds change (player already exists)
  useEffect(() => {
    const player = playerRef.current
    if (!player) return

    if (player.readyState() >= 1) {
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
})

export default VideoPreview
