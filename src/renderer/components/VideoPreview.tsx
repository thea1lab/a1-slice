import React, { useRef, useEffect, useState, useCallback } from 'react'
import { paddedPreviewRange, previewCoversRange, toPreviewSrc } from '../../shared/previewUrl'
import { containRect, cropRect, snapCropCenter, clamp } from '../../shared/crop'
import type { ClipCrop } from '../../shared/types'
import { DEFAULT_CROP } from '../../shared/types'

interface VideoPreviewProps {
  videoPath: string
  startMs: number
  endMs: number
  videoDurationMs?: number
  crop?: ClipCrop
  onCropChange?: (crop: ClipCrop) => void
  onPlayheadMs?: (ms: number) => void
  seekToMs?: number
  seekNonce?: number
  editor?: boolean
  className?: string
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
  videoDurationMs,
  crop = DEFAULT_CROP,
  onCropChange,
  onPlayheadMs,
  seekToMs,
  seekNonce,
  editor = false,
  className
}: VideoPreviewProps): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null)
  const videoRef = useRef<HTMLVideoElement>(null)
  const coverRef = useRef<Cover | null>(null)
  const boundsRef = useRef({ startMs, endMs })
  boundsRef.current = { startMs, endMs }
  const cropRef = useRef(crop)
  cropRef.current = crop
  const playheadRef = useRef(onPlayheadMs)
  playheadRef.current = onPlayheadMs
  const onCropChangeRef = useRef(onCropChange)
  onCropChangeRef.current = onCropChange
  const dragRef = useRef<
    | { mode: 'pan'; x: number; y: number; cx: number; cy: number }
    | { mode: 'zoom' }
    | null
  >(null)

  const [inView, setInView] = useState(editor)
  const [src, setSrc] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [playing, setPlaying] = useState(false)
  const [content, setContent] = useState({ x: 0, y: 0, w: 0, h: 0 })

  const measure = useCallback((): void => {
    const el = containerRef.current
    const video = videoRef.current
    if (!el) return
    const cw = el.clientWidth
    const ch = el.clientHeight
    const vw = video?.videoWidth ?? 0
    const vh = video?.videoHeight ?? 0
    setContent(containRect(cw, ch, vw, vh))
  }, [])

  useEffect(() => {
    if (editor) {
      setInView(true)
      return
    }
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
  }, [editor])

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

    const emitPlayhead = (): void => {
      const cover = coverRef.current
      if (!cover || !playheadRef.current) return
      playheadRef.current(cover.fileStartMs + video.currentTime * 1000)
    }

    const onLoadedMetadata = (): void => {
      video.currentTime = localStart()
      measure()
    }

    const onTimeUpdate = (): void => {
      emitPlayhead()
      if (video.paused) return
      if (video.currentTime >= localEnd()) {
        video.currentTime = localStart()
        void video.play()
      }
    }

    const onEnded = (): void => {
      video.currentTime = localStart()
      if (editor) void video.play()
    }

    const onPlay = (): void => setPlaying(true)
    const onPause = (): void => setPlaying(false)

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
    video.addEventListener('play', onPlay)
    video.addEventListener('pause', onPause)
    video.addEventListener('error', onMediaError)
    if (video.readyState >= 1) onLoadedMetadata()

    return () => {
      video.removeEventListener('loadedmetadata', onLoadedMetadata)
      video.removeEventListener('timeupdate', onTimeUpdate)
      video.removeEventListener('ended', onEnded)
      video.removeEventListener('play', onPlay)
      video.removeEventListener('pause', onPause)
      video.removeEventListener('error', onMediaError)
    }
  }, [src, editor, measure])

  useEffect(() => {
    const video = videoRef.current
    const cover = coverRef.current
    if (!video || !cover || video.readyState < 1) return
    if (seekNonce == null || seekToMs == null) return
    const t = Math.max(0, (seekToMs - cover.fileStartMs) / 1000)
    if (Math.abs(video.currentTime - t) > 0.05) {
      video.currentTime = t
    }
  }, [seekToMs, seekNonce])

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const ro = new ResizeObserver(() => measure())
    ro.observe(el)
    measure()
    return () => ro.disconnect()
  }, [measure, src])

  const togglePlay = (): void => {
    const video = videoRef.current
    if (!video) return
    if (video.paused) void video.play()
    else video.pause()
  }

  const setZoom = (next: number): void => {
    if (!onCropChange || crop.ratio === 'original') return
    onCropChange(snapCropCenter({ ...crop, zoom: clamp(next, 0, 1) }))
  }

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    if (!editor || !onCropChange || crop.ratio === 'original') return
    const target = e.target as HTMLElement
    const corner = target.dataset.corner
    if (corner) {
      dragRef.current = { mode: 'zoom' }
    } else {
      dragRef.current = {
        mode: 'pan',
        x: e.clientX,
        y: e.clientY,
        cx: crop.cx,
        cy: crop.cy
      }
    }
    e.currentTarget.setPointerCapture(e.pointerId)
  }

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>): void => {
    const drag = dragRef.current
    if (!drag || !onCropChange) return
    if (drag.mode === 'pan') {
      if (content.w <= 0 || content.h <= 0) return
      const dx = (e.clientX - drag.x) / content.w
      const dy = (e.clientY - drag.y) / content.h
      onCropChange(snapCropCenter({ ...cropRef.current, cx: drag.cx + dx, cy: drag.cy + dy }))
      return
    }
    const rect = containerRef.current?.getBoundingClientRect()
    if (!rect || content.w <= 0) return
    const maxBox = cropRect({ ...cropRef.current, zoom: 0 }, content.w, content.h)
    const cx = content.x + maxBox.x + maxBox.w / 2
    const cy = content.y + maxBox.y + maxBox.h / 2
    const px = e.clientX - rect.left
    const py = e.clientY - rect.top
    const dist = Math.hypot(px - cx, py - cy)
    const maxDist = Math.hypot(maxBox.w / 2, maxBox.h / 2)
    const minDist = maxDist * 0.42
    const zoom = 1 - (clamp(dist, minDist, maxDist) - minDist) / Math.max(1e-6, maxDist - minDist)
    onCropChange(snapCropCenter({ ...cropRef.current, zoom: clamp(zoom, 0, 1) }))
  }

  const onPointerUp = (): void => {
    dragRef.current = null
  }

  useEffect(() => {
    const el = containerRef.current
    if (!el || !editor) return
    const onWheel = (e: WheelEvent): void => {
      if (cropRef.current.ratio === 'original' || !onCropChangeRef.current) return
      e.preventDefault()
      const next = clamp(cropRef.current.zoom + (e.deltaY > 0 ? -0.06 : 0.06), 0, 1)
      onCropChangeRef.current(snapCropCenter({ ...cropRef.current, zoom: next }))
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  }, [editor])

  const cropped = editor && crop.ratio !== 'original'
  const box = cropped ? cropRect(crop, content.w, content.h) : null

  return (
    <div
      ref={containerRef}
      className={`relative w-full bg-black aspect-video overflow-hidden ${
        editor
          ? cropped
            ? 'cursor-grab active:cursor-grabbing select-none'
            : 'select-none'
          : 'rounded-lg'
      } ${className ?? ''}`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
    >
      {src && (
        <video
          ref={videoRef}
          src={src}
          controls={!editor}
          playsInline
          preload="auto"
          className="w-full h-full object-contain bg-black"
        />
      )}
      {box && content.w > 0 && (
        <div
          className="absolute border-2 border-accent pointer-events-none shadow-[0_0_0_9999px_rgba(0,0,0,0.62)]"
          style={{
            left: content.x + box.x,
            top: content.y + box.y,
            width: box.w,
            height: box.h
          }}
        >
          <span className="absolute -top-1.5 -left-1.5 w-3.5 h-3.5 rounded-sm bg-accent border-2 border-bg-base pointer-events-auto cursor-nwse-resize" data-corner="tl" />
          <span className="absolute -top-1.5 -right-1.5 w-3.5 h-3.5 rounded-sm bg-accent border-2 border-bg-base pointer-events-auto cursor-nesw-resize" data-corner="tr" />
          <span className="absolute -bottom-1.5 -left-1.5 w-3.5 h-3.5 rounded-sm bg-accent border-2 border-bg-base pointer-events-auto cursor-nesw-resize" data-corner="bl" />
          <span className="absolute -bottom-1.5 -right-1.5 w-3.5 h-3.5 rounded-sm bg-accent border-2 border-bg-base pointer-events-auto cursor-nwse-resize" data-corner="br" />
        </div>
      )}
      {editor && (
        <button
          type="button"
          onPointerDown={(e) => e.stopPropagation()}
          onClick={(e) => {
            e.stopPropagation()
            togglePlay()
          }}
          className="absolute left-3 bottom-3 z-10 w-9 h-9 rounded-full border border-white/15 bg-bg-card/80 text-neutral-100 grid place-items-center hover:border-accent/50 hover:text-accent"
          aria-label={playing ? 'Pause' : 'Play'}
        >
          {playing ? (
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <rect x="2" y="1.5" width="2.5" height="9" rx="0.5" />
              <rect x="7.5" y="1.5" width="2.5" height="9" rx="0.5" />
            </svg>
          ) : (
            <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
              <path d="M3 1.5v9l7.5-4.5L3 1.5z" />
            </svg>
          )}
        </button>
      )}
      {editor && cropped && (
        <div className="absolute right-3 bottom-3 z-10 flex flex-col items-center gap-1.5">
          <button
            type="button"
            disabled={crop.zoom >= 1}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation()
              setZoom(crop.zoom + 0.1)
            }}
            className="w-9 h-9 rounded-full border border-white/15 bg-bg-card/80 text-neutral-100 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-35"
            aria-label="Zoom in"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 3.5v9M3.5 8h9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </button>
          <div className="text-[10px] font-semibold text-neutral-200 bg-bg-card/80 border border-white/15 rounded-full px-2 py-0.5 min-w-9 text-center">
            {crop.zoom < 0.04 ? 'Fit' : `${Math.round(crop.zoom * 100)}%`}
          </div>
          <button
            type="button"
            disabled={crop.zoom <= 0}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation()
              setZoom(crop.zoom - 0.1)
            }}
            className="w-9 h-9 rounded-full border border-white/15 bg-bg-card/80 text-neutral-100 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-35"
            aria-label="Zoom out"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M3.5 8h9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </button>
        </div>
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
