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

const SCRUB_EPSILON_SEC = 0.04

/**
 * One seek has to finish and paint before the next currentTime assignment.
 * While it is in flight, keep only the latest requested time.
 */
export function planScrub(
  currentTime: number,
  requested: number,
  busy: boolean,
  epsilon = SCRUB_EPSILON_SEC
): { seekTo: number | null; queued: number | null } {
  if (!Number.isFinite(requested)) return { seekTo: null, queued: null }
  if (busy) return { seekTo: null, queued: requested }
  if (!Number.isFinite(currentTime) || Math.abs(currentTime - requested) > epsilon) {
    return { seekTo: requested, queued: null }
  }
  return { seekTo: null, queued: null }
}
