import React, { useRef, useEffect, useState } from 'react'
import { paddedPreviewRange, previewCoversRange, toPreviewSrc } from '../../shared/previewUrl'

interface VideoPreviewProps {
  videoPath: string
  startMs: number
  endMs: number
  videoDurationMs?: number
}

interface Cover {
  fileStartMs: number
  fileEndMs: number
  src: string
  previewPath: string
}

const VideoPreview = React.memo(function VideoPreview({
  videoPath,
  startMs,
  endMs,
  videoDurationMs
}: VideoPreviewProps): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null)
  const videoRef = useRef<HTMLVideoElement>(null)
  const coverRef = useRef<Cover | null>(null)
  const boundsRef = useRef({ startMs, endMs })
  boundsRef.current = { startMs, endMs }

  const [inView, setInView] = useState(false)
  const [src, setSrc] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const io = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) setInView(true)
      },
      { rootMargin: '160px' }
    )
    io.observe(el)
    return () => io.disconnect()
  }, [])

  useEffect(() => {
    if (!inView || !videoPath || endMs <= startMs) return

    const existing = coverRef.current
    if (existing && previewCoversRange(existing, startMs, endMs)) {
      return
    }

    let cancelled = false
    let adopted = false
    let createdPath: string | null = null
    let createdBlob: string | null = null
    setBusy(true)
    setError(null)

    const range = paddedPreviewRange(startMs, endMs, videoDurationMs)

    void (async () => {
      const result = await window.api.createClipPreview(
        videoPath,
        range.fileStartMs,
        range.fileEndMs
      )
      if (cancelled) {
        if (result.previewPath) void window.api.releaseClipPreview(result.previewPath)
        return
      }
      if (!result.success || !result.previewPath) {
        setSrc(null)
        setError(result.error || 'Preview failed')
        setBusy(false)
        return
      }
      createdPath = result.previewPath

      let previewSrc = toPreviewSrc(result.previewPath)
      const bytes = await window.api.readClipPreview(result.previewPath)
      if (cancelled) {
        void window.api.releaseClipPreview(result.previewPath)
        return
      }
      if (bytes && bytes.byteLength > 0) {
        const blobUrl = URL.createObjectURL(
          new Blob([bytes as BlobPart], { type: 'video/mp4' })
        )
        createdBlob = blobUrl
        previewSrc = blobUrl
      }

      const previous = coverRef.current
      coverRef.current = {
        fileStartMs: range.fileStartMs,
        fileEndMs: range.fileEndMs,
        src: previewSrc,
        previewPath: result.previewPath
      }
      adopted = true
      setSrc(previewSrc)
      setError(null)
      setBusy(false)
      if (previous) {
        if (previous.src.startsWith('blob:')) URL.revokeObjectURL(previous.src)
        void window.api.releaseClipPreview(previous.previewPath)
      }
    })().catch((err: unknown) => {
      if (cancelled) return
      setError(err instanceof Error ? err.message : 'Preview failed')
      setBusy(false)
    })

    return () => {
      cancelled = true
      if (adopted) return
      if (createdBlob) URL.revokeObjectURL(createdBlob)
      if (createdPath) void window.api.releaseClipPreview(createdPath)
    }
  }, [inView, videoPath, startMs, endMs, videoDurationMs])

  useEffect(() => {
    return () => {
      const cover = coverRef.current
      if (!cover) return
      if (cover.src.startsWith('blob:')) URL.revokeObjectURL(cover.src)
      void window.api.releaseClipPreview(cover.previewPath)
      coverRef.current = null
    }
  }, [])

  useEffect(() => {
    const video = videoRef.current
    if (!video || !src) return

    const localStart = (): number => {
      const cover = coverRef.current
      if (!cover) return 0
      return Math.max(0, (boundsRef.current.startMs - cover.fileStartMs) / 1000)
    }
    const localEnd = (): number => {
      const cover = coverRef.current
      if (!cover) return Number.isFinite(video.duration) ? video.duration : 0
      return Math.max(localStart() + 0.2, (boundsRef.current.endMs - cover.fileStartMs) / 1000)
    }

    const onLoadedMetadata = (): void => {
      video.currentTime = localStart()
    }

    const onTimeUpdate = (): void => {
      if (video.paused) return
      if (video.currentTime >= localEnd()) {
        video.pause()
        video.currentTime = localStart()
      }
    }

    const onEnded = (): void => {
      video.currentTime = localStart()
    }

    const onMediaError = (): void => {
      const cover = coverRef.current
      if (!cover || !cover.src.startsWith('blob:')) return
      const fallback = toPreviewSrc(cover.previewPath)
      cover.src = fallback
      setSrc(fallback)
    }

    video.addEventListener('loadedmetadata', onLoadedMetadata)
    video.addEventListener('timeupdate', onTimeUpdate)
    video.addEventListener('ended', onEnded)
    video.addEventListener('error', onMediaError)
    if (video.readyState >= 1) onLoadedMetadata()

    return () => {
      video.removeEventListener('loadedmetadata', onLoadedMetadata)
      video.removeEventListener('timeupdate', onTimeUpdate)
      video.removeEventListener('ended', onEnded)
      video.removeEventListener('error', onMediaError)
    }
  }, [src])

  useEffect(() => {
    const video = videoRef.current
    const cover = coverRef.current
    if (!video || !cover || video.readyState < 1) return
    const t = Math.max(0, (startMs - cover.fileStartMs) / 1000)
    if (Math.abs(video.currentTime - t) > 0.2) {
      video.currentTime = t
    }
  }, [startMs, endMs])

  return (
    <div
      ref={containerRef}
      className="relative w-full rounded-lg bg-black aspect-video overflow-hidden"
    >
      {src && (
        <video
          ref={videoRef}
          src={src}
          controls
          playsInline
          preload="auto"
          className="w-full h-full object-contain bg-black"
        />
      )}
      {busy && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/55 text-xs text-neutral-400">
          Cutting preview…
        </div>
      )}
      {error && !busy && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-3 text-center text-xs text-neutral-400">
          {error}
        </div>
      )}
    </div>
  )
})

export default VideoPreview
