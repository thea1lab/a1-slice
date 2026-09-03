import React, { useRef, useEffect, useState, useCallback, useImperativeHandle } from 'react'
import {
  paddedPreviewRange,
  playbackTimeOnPlay,
  previewCoversRange,
  previewLocalTime,
  shouldPublishPlayhead,
  toPreviewSrc
} from '../../shared/previewUrl'
import { containRect, cropRect, snapCropCenter, clamp } from '../../shared/crop'
import type { ClipCrop } from '../../shared/types'
import { DEFAULT_CROP } from '../../shared/types'

export interface VideoPreviewHandle {
  togglePlay: () => void
  play: () => void
  pause: () => void
  seek: (ms: number) => void
  setVolume: (volume: number) => void
}

interface VideoPreviewProps {
  videoPath: string
  startMs: number
  endMs: number
  playStartMs?: number
  playEndMs?: number
  videoDurationMs?: number
  crop?: ClipCrop
  volume?: number
  onCropChange?: (crop: ClipCrop) => void
  onPlayheadMs?: (ms: number) => void
  onPlayingChange?: (playing: boolean) => void
  onMediaInfo?: (info: { width: number; height: number }) => void
  seekToMs?: number
  seekNonce?: number
  editor?: boolean
  className?: string
  ref?: React.Ref<VideoPreviewHandle>
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
  playStartMs,
  playEndMs,
  videoDurationMs,
  crop = DEFAULT_CROP,
  volume = 1,
  onCropChange,
  onPlayheadMs,
  onPlayingChange,
  onMediaInfo,
  seekToMs,
  seekNonce,
  editor = false,
  className,
  ref
}: VideoPreviewProps): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null)
  const videoRef = useRef<HTMLVideoElement>(null)
  const coverRef = useRef<Cover | null>(null)
  const boundsRef = useRef({ startMs, endMs })
  boundsRef.current = { startMs, endMs }
  const playBoundsRef = useRef({
    startMs: playStartMs ?? startMs,
    endMs: playEndMs ?? endMs
  })
  playBoundsRef.current = {
    startMs: playStartMs ?? startMs,
    endMs: playEndMs ?? endMs
  }
  const cropRef = useRef(crop)
  cropRef.current = crop
  const playheadRef = useRef(onPlayheadMs)
  playheadRef.current = onPlayheadMs
  const onCropChangeRef = useRef(onCropChange)
  onCropChangeRef.current = onCropChange
  const onPlayingChangeRef = useRef(onPlayingChange)
  onPlayingChangeRef.current = onPlayingChange
  const onMediaInfoRef = useRef(onMediaInfo)
  onMediaInfoRef.current = onMediaInfo
  const volumeRef = useRef(volume)
  volumeRef.current = volume
  const previewJobRef = useRef(0)
  const loopAtOutRef = useRef(true)
  const snapInOnStartRef = useRef(false)
  const holdPlayheadRef = useRef(true)
  const seekToMsRef = useRef(seekToMs)
  seekToMsRef.current = seekToMs
  const dragRef = useRef<
    | { mode: 'pan'; x: number; y: number; cx: number; cy: number }
    | { mode: 'zoom' }
    | null
  >(null)

  const [inView, setInView] = useState(editor)
  const [src, setSrc] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [content, setContent] = useState({ x: 0, y: 0, w: 0, h: 0 })
  const [previewPercent, setPreviewPercent] = useState(0)
  const [frameReady, setFrameReady] = useState(false)
  const contentRef = useRef(content)
  contentRef.current = content

  const snap = (next: ClipCrop): ClipCrop => {
    const { w, h } = contentRef.current
    return snapCropCenter(next, w > 0 ? w : 1, h > 0 ? h : 1)
  }

  const measure = useCallback((): void => {
    const el = containerRef.current
    const video = videoRef.current
    if (!el) return
    const cw = el.clientWidth
    const ch = el.clientHeight
    const vw = video?.videoWidth ?? 0
    const vh = video?.videoHeight ?? 0
    setContent(containRect(cw, ch, vw, vh))
    if (vw > 0 && vh > 0) {
      setFrameReady(true)
      setBusy(false)
      onMediaInfoRef.current?.({ width: vw, height: vh })
    }
  }, [])

  const clipStartSec = (): number => {
    const cover = coverRef.current
    if (!cover) return 0
    return previewLocalTime(boundsRef.current.startMs, cover.fileStartMs)
  }

  const clipEndSec = (): number => {
    const video = videoRef.current
    const cover = coverRef.current
    if (!cover) return video && Number.isFinite(video.duration) ? video.duration : 0
    const end = previewLocalTime(boundsRef.current.endMs, cover.fileStartMs)
    const duration = video && Number.isFinite(video.duration) ? video.duration : end
    return Math.max(clipStartSec() + 0.2, Math.min(end, duration))
  }

  const windowStartSec = (): number => {
    const cover = coverRef.current
    if (!cover) return 0
    return previewLocalTime(playBoundsRef.current.startMs, cover.fileStartMs)
  }

  const windowEndSec = (): number => {
    const video = videoRef.current
    const cover = coverRef.current
    if (!cover) return video && Number.isFinite(video.duration) ? video.duration : 0
    const end = previewLocalTime(playBoundsRef.current.endMs, cover.fileStartMs)
    const duration = video && Number.isFinite(video.duration) ? video.duration : end
    return Math.max(windowStartSec() + 0.2, Math.min(end, duration))
  }

  const seekToSourceMs = (ms: number): void => {
    const video = videoRef.current
    const cover = coverRef.current
    if (!video || !cover) return
    snapInOnStartRef.current = false
    let next = previewLocalTime(ms, cover.fileStartMs)
    if (Number.isFinite(video.duration) && video.duration > 0) {
      next = Math.min(next, Math.max(0, video.duration - 0.05))
    }
    if (Math.abs(video.currentTime - next) > 0.04) {
      holdPlayheadRef.current = true
      video.currentTime = next
    } else {
      holdPlayheadRef.current = false
    }
    loopAtOutRef.current = ms < boundsRef.current.endMs
  }

  const startPlayback = (): void => {
    const video = videoRef.current
    if (!video) return
    const inPt = clipStartSec()
    const outPt = clipEndSec()
    const t = playbackTimeOnPlay(video.currentTime, inPt, outPt)
    snapInOnStartRef.current = t === inPt && video.currentTime <= 0.05
    if (Math.abs(video.currentTime - t) > 0.05) {
      video.currentTime = t
    }
    void video.play()
  }

  const togglePlay = useCallback((): void => {
    const video = videoRef.current
    if (!video) return
    if (video.paused) startPlayback()
    else video.pause()
  }, [])

  useImperativeHandle(ref, () => ({
    togglePlay,
    play: () => startPlayback(),
    pause: () => {
      videoRef.current?.pause()
    },
    seek: seekToSourceMs,
    setVolume: (next: number) => {
      const video = videoRef.current
      if (!video) return
      video.volume = clamp(next, 0, 1)
      video.muted = next <= 0
    }
  }))

  useEffect(() => {
    const subscribe = window.api.onPreviewProgress
    if (typeof subscribe !== 'function') return
    return subscribe((update) => {
      if (update.previewId !== String(previewJobRef.current)) return
      setPreviewPercent(clamp(Math.round(update.percent), 0, 100))
    })
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
    setBusy(true)
    setFrameReady(false)
    setPreviewPercent(0)
    setError(null)

    const range = paddedPreviewRange(startMs, endMs, videoDurationMs)
    const previewId = String(++previewJobRef.current)

    void (async () => {
      const result = await window.api.createClipPreview(
        videoPath,
        range.fileStartMs,
        range.fileEndMs,
        previewId
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
      const previewSrc = toPreviewSrc(result.previewPath)

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
    holdPlayheadRef.current = true

    const emitPlayhead = (): void => {
      if (!shouldPublishPlayhead(video.seeking, holdPlayheadRef.current)) return
      const cover = coverRef.current
      if (!cover || !playheadRef.current) return
      playheadRef.current(cover.fileStartMs + video.currentTime * 1000)
    }

    const applyInPoint = (): void => {
      const cover = coverRef.current
      const ms = seekToMsRef.current ?? boundsRef.current.startMs
      const t = cover ? previewLocalTime(ms, cover.fileStartMs) : clipStartSec()
      if (Math.abs(video.currentTime - t) > 0.05) {
        holdPlayheadRef.current = true
        video.currentTime = t
      } else {
        holdPlayheadRef.current = false
        emitPlayhead()
      }
    }

    const onLoadedMetadata = (): void => {
      applyInPoint()
      const vol = volumeRef.current
      video.volume = clamp(vol, 0, 1)
      video.muted = vol <= 0
      measure()
    }

    const onTimeUpdate = (): void => {
      if (!shouldPublishPlayhead(video.seeking, holdPlayheadRef.current)) return
      if (snapInOnStartRef.current && video.currentTime < clipStartSec() - 0.05) {
        video.currentTime = clipStartSec()
        return
      }
      if (video.currentTime >= clipStartSec() - 0.05) {
        snapInOnStartRef.current = false
      }
      emitPlayhead()
      if (video.paused) return
      const loopAt = loopAtOutRef.current ? clipEndSec() : windowEndSec()
      if (video.currentTime >= loopAt) {
        video.currentTime = clipStartSec()
        loopAtOutRef.current = true
        void video.play()
      }
    }

    const onSeeked = (): void => {
      holdPlayheadRef.current = false
      emitPlayhead()
    }

    const onEnded = (): void => {
      video.currentTime = clipStartSec()
      if (editor) void video.play()
    }

    const onPlay = (): void => {
      if (video.seeking) {
        onPlayingChangeRef.current?.(true)
        return
      }
      const inPt = clipStartSec()
      const outPt = clipEndSec()
      const t = playbackTimeOnPlay(video.currentTime, inPt, outPt)
      if (Math.abs(video.currentTime - t) > 0.05) {
        holdPlayheadRef.current = true
        video.currentTime = t
      }
      loopAtOutRef.current = video.currentTime < outPt - 0.05
      onPlayingChangeRef.current?.(true)
    }
    const onPause = (): void => {
      onPlayingChangeRef.current?.(false)
    }

    const onLoadedData = (): void => {
      if (video.paused) applyInPoint()
    }

    video.addEventListener('loadedmetadata', onLoadedMetadata)
    video.addEventListener('loadeddata', onLoadedData)
    video.addEventListener('timeupdate', onTimeUpdate)
    video.addEventListener('seeked', onSeeked)
    video.addEventListener('ended', onEnded)
    video.addEventListener('play', onPlay)
    video.addEventListener('pause', onPause)
    if (video.readyState >= 1) onLoadedMetadata()

    return () => {
      video.removeEventListener('loadedmetadata', onLoadedMetadata)
      video.removeEventListener('loadeddata', onLoadedData)
      video.removeEventListener('timeupdate', onTimeUpdate)
      video.removeEventListener('seeked', onSeeked)
      video.removeEventListener('ended', onEnded)
      video.removeEventListener('play', onPlay)
      video.removeEventListener('pause', onPause)
    }
  }, [src, editor, measure])

  useEffect(() => {
    const video = videoRef.current
    if (!video) return
    video.volume = clamp(volume, 0, 1)
    video.muted = volume <= 0
  }, [volume])

  useEffect(() => {
    if (seekNonce == null || seekToMs == null) return
    seekToSourceMs(seekToMs)
  }, [seekToMs, seekNonce, src])

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const ro = new ResizeObserver(() => measure())
    ro.observe(el)
    measure()
    return () => ro.disconnect()
  }, [measure, src])

  const setZoom = (next: number): void => {
    if (!onCropChange || crop.ratio === 'original') return
    onCropChange(snap({ ...crop, zoom: clamp(next, 0, 1) }))
  }

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
    if (!editor || !onCropChange || crop.ratio === 'original' || busy || !frameReady) return
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
      onCropChange(snap({ ...cropRef.current, cx: drag.cx + dx, cy: drag.cy + dy }))
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
    onCropChange(snap({ ...cropRef.current, zoom: clamp(zoom, 0, 1) }))
  }

  const onPointerUp = (): void => {
    dragRef.current = null
  }

  useEffect(() => {
    const el = containerRef.current
    if (!el || !editor) return
    const onWheel = (e: WheelEvent): void => {
      if (cropRef.current.ratio === 'original' || !onCropChangeRef.current || busy || !frameReady) return
      e.preventDefault()
      const next = clamp(cropRef.current.zoom + (e.deltaY > 0 ? -0.06 : 0.06), 0, 1)
      onCropChangeRef.current(snap({ ...cropRef.current, zoom: next }))
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  }, [editor, busy, frameReady])

  const cropped = editor && crop.ratio !== 'original' && frameReady && !busy
  const box = cropped ? cropRect(crop, content.w, content.h) : null

  return (
    <div
      ref={containerRef}
      className={`relative w-full bg-black overflow-hidden ${
        editor
          ? `h-full min-h-0 ${cropped ? 'cursor-grab active:cursor-grabbing select-none' : 'select-none cursor-pointer'}`
          : 'aspect-video rounded-lg'
      } ${className ?? ''}`}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onDoubleClick={(e) => {
        e.preventDefault()
        const root = containerRef.current?.closest('[data-player-root]')
        if (root instanceof HTMLElement) {
          if (document.fullscreenElement) void document.exitFullscreen()
          else void root.requestFullscreen()
        }
      }}
      onClick={(e) => {
        if (!editor || cropped) return
        if (e.target !== videoRef.current && e.target !== e.currentTarget) return
        togglePlay()
      }}
    >
      {src && (
        <video
          ref={videoRef}
          src={src}
          controls={!editor}
          playsInline
          preload="auto"
          onError={() => {
            setBusy(false)
            setError('This video could not be played')
          }}
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
      {editor && cropped && (
        <div className="absolute right-3 bottom-3 z-10 flex items-center gap-1.5">
          <button
            type="button"
            disabled={crop.zoom <= 0}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation()
              setZoom(crop.zoom - 0.1)
            }}
            className="w-8 h-8 rounded-md border border-white/15 bg-black/70 text-neutral-100 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-35"
            aria-label="Zoom out"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M3.5 8h9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </button>
          <div className="text-[10px] font-semibold text-neutral-200 bg-black/70 border border-white/15 rounded-md px-2 py-1 min-w-9 text-center">
            {crop.zoom < 0.04 ? 'Fit' : `${Math.round(crop.zoom * 100)}%`}
          </div>
          <button
            type="button"
            disabled={crop.zoom >= 1}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.stopPropagation()
              setZoom(crop.zoom + 0.1)
            }}
            className="w-8 h-8 rounded-md border border-white/15 bg-black/70 text-neutral-100 grid place-items-center hover:border-accent/50 hover:text-accent disabled:opacity-35"
            aria-label="Zoom in"
          >
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <path d="M8 3.5v9M3.5 8h9" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
            </svg>
          </button>
        </div>
      )}
      {busy && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="w-64 max-w-[70%] text-center">
            <p className="text-sm text-white/80">Preparing clip</p>
            <p className="mt-1 text-[11px] text-white/45">
              Making a player copy at the original size. This can take a bit on long or 4K files.
            </p>
            <div className="mt-3 h-px bg-white/15 overflow-hidden">
              <div
                className="h-full bg-accent transition-[width] duration-300"
                style={{ width: `${Math.max(4, previewPercent)}%` }}
              />
            </div>
            {previewPercent > 0 && (
              <p className="mt-2 text-[11px] tabular-nums text-white/40">{previewPercent}%</p>
            )}
          </div>
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
