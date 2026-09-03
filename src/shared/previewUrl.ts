function utf8ToBase64Url(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let bin = ''
  for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i])
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

function base64UrlToUtf8(token: string): string {
  const padded = token.replace(/-/g, '+').replace(/_/g, '/')
  const pad = padded.length % 4 === 0 ? '' : '='.repeat(4 - (padded.length % 4))
  const bin = atob(padded + pad)
  const bytes = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
  return new TextDecoder().decode(bytes)
}

export const PREVIEW_PAD_MS = 12_000

export function paddedPreviewRange(
  startMs: number,
  endMs: number,
  videoDurationMs?: number
): { fileStartMs: number; fileEndMs: number } {
  const fileStartMs = Math.max(0, startMs - PREVIEW_PAD_MS)
  let fileEndMs = endMs + PREVIEW_PAD_MS
  if (videoDurationMs && videoDurationMs > 0) {
    fileEndMs = Math.min(videoDurationMs, fileEndMs)
  }
  if (fileEndMs <= fileStartMs) {
    fileEndMs = fileStartMs + Math.max(200, endMs - startMs)
  }
  return { fileStartMs, fileEndMs }
}

export function previewCoversRange(
  cover: { fileStartMs: number; fileEndMs: number },
  startMs: number,
  endMs: number
): boolean {
  return startMs >= cover.fileStartMs && endMs <= cover.fileEndMs
}

export function previewLocalTime(sourceMs: number, fileStartMs: number): number {
  return Math.max(0, (sourceMs - fileStartMs) / 1000)
}

const PLAY_SNAP_EPS = 0.05

export function playbackTimeOnPlay(
  currentTime: number,
  inPoint: number,
  outPoint: number
): number {
  if (!Number.isFinite(currentTime)) return inPoint
  if (currentTime <= PLAY_SNAP_EPS && inPoint > PLAY_SNAP_EPS) return inPoint
  if (currentTime >= outPoint - PLAY_SNAP_EPS) return inPoint
  return currentTime
}

export function shouldPublishPlayhead(seeking: boolean, holding: boolean): boolean {
  return !seeking && !holding
}

export function pointerToSourceMs(
  clientX: number,
  trackLeft: number,
  trackWidth: number,
  srcStart: number,
  srcEnd: number
): number {
  if (!(trackWidth > 0) || !Number.isFinite(trackWidth)) return srcStart
  const x = Math.min(trackWidth, Math.max(0, clientX - trackLeft))
  return srcStart + (x / trackWidth) * Math.max(1, srcEnd - srcStart)
}

export function toPreviewSrc(filePath: string): string {
  const posix = filePath.replace(/\\/g, '/')
  return `a1slice://preview/${utf8ToBase64Url(posix)}`
}

export function pathFromPreviewUrl(requestUrl: string): string | null {
  try {
    const url = new URL(requestUrl)
    const fromQuery = url.searchParams.get('path')
    if (fromQuery) return fromQuery
    const token = url.pathname.replace(/^\//, '')
    if (!token) return null
    return base64UrlToUtf8(token)
  } catch {
    return null
  }
}
