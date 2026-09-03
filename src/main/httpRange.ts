export interface ByteRange {
  start: number
  end: number
}

export function parseByteRange(
  header: string | null | undefined,
  size: number
): ByteRange | null {
  if (!header || size <= 0) return null
  const match = /^bytes=(\d*)-(\d*)$/i.exec(header.trim())
  if (!match) return null

  const hasStart = match[1] !== ''
  const hasEnd = match[2] !== ''
  if (!hasStart && !hasEnd) return null

  let start: number
  let end: number
  if (!hasStart) {
    const suffix = parseInt(match[2], 10)
    if (!Number.isFinite(suffix) || suffix <= 0) return null
    start = Math.max(0, size - suffix)
    end = size - 1
  } else {
    start = parseInt(match[1], 10)
    end = hasEnd ? parseInt(match[2], 10) : size - 1
  }

  if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0 || start >= size) {
    return null
  }
  if (end >= size) end = size - 1
  if (end < start) return null
  return { start, end }
}

export function rangeResponseMeta(
  size: number,
  range: ByteRange | null,
  mime: string
): { status: number; headers: Record<string, string> } {
  if (!range) {
    return {
      status: 200,
      headers: {
        'Content-Type': mime,
        'Content-Length': String(size),
        'Accept-Ranges': 'bytes'
      }
    }
  }
  return {
    status: 206,
    headers: {
      'Content-Type': mime,
      'Content-Length': String(range.end - range.start + 1),
      'Content-Range': `bytes ${range.start}-${range.end}/${size}`,
      'Accept-Ranges': 'bytes'
    }
  }
}

const VIDEO_MIME: Record<string, string> = {
  '.mp4': 'video/mp4',
  '.mov': 'video/quicktime',
  '.webm': 'video/webm',
  '.mkv': 'video/x-matroska',
  '.avi': 'video/x-msvideo'
}

export function videoMimeForExt(ext: string): string {
  return VIDEO_MIME[ext.toLowerCase()] ?? 'application/octet-stream'
}
