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
