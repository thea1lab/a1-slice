export function formatPlaybackClock(ms: number): string {
  const safe = Number.isFinite(ms) ? Math.max(0, ms) : 0
  const totalSeconds = Math.floor(safe / 1000)
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  const body = `${minutes}:${String(seconds).padStart(2, '0')}`
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}` : body
}

export function playbackMsAt(
  clientX: number,
  left: number,
  width: number,
  startMs: number,
  endMs: number
): number {
  const span = Math.max(0, endMs - startMs)
  if (!(width > 0) || !Number.isFinite(clientX)) return startMs
  const ratio = Math.min(1, Math.max(0, (clientX - left) / width))
  return startMs + ratio * span
}
