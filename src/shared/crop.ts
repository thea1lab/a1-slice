import type { ClipCrop, CropRatio } from './types'
import { DEFAULT_CROP } from './types'

export const RATIO_PAIR: Record<Exclude<CropRatio, 'original'>, readonly [number, number]> = {
  '4:3': [4, 3],
  '9:16': [9, 16],
  '1:1': [1, 1]
}

export const CROP_OUTPUT_SIZE: Record<Exclude<CropRatio, 'original'>, readonly [number, number]> = {
  '4:3': [1440, 1080],
  '9:16': [1080, 1920],
  '1:1': [1080, 1080]
}

export const MAX_ZOOM_SCALE = 0.42

export function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n))
}

export function even(n: number): number {
  return Math.max(2, Math.floor(n / 2) * 2)
}

export function cropRect(
  crop: ClipCrop,
  vw: number,
  vh: number
): { x: number; y: number; w: number; h: number } {
  if (crop.ratio === 'original' || vw <= 0 || vh <= 0) {
    return { x: 0, y: 0, w: vw, h: vh }
  }
  const [rw, rh] = RATIO_PAIR[crop.ratio]
  let w = vw
  let h = w * (rh / rw)
  if (h > vh) {
    h = vh
    w = h * (rw / rh)
  }
  const zoom = clamp(crop.zoom, 0, 1)
  const scale = 1 - zoom * (1 - MAX_ZOOM_SCALE)
  w *= scale
  h *= scale
  let x = crop.cx * vw - w / 2
  let y = crop.cy * vh - h / 2
  x = clamp(x, 0, Math.max(0, vw - w))
  y = clamp(y, 0, Math.max(0, vh - h))
  return { x, y, w, h }
}

export function snapCropCenter(crop: ClipCrop, vw = 1, vh = 1): ClipCrop {
  const r = cropRect(crop, vw, vh)
  if (vw <= 0 || vh <= 0) return crop
  return {
    ...crop,
    cx: (r.x + r.w / 2) / vw,
    cy: (r.y + r.h / 2) / vh
  }
}

export function ffmpegCropFilter(
  crop: ClipCrop,
  vw: number,
  vh: number
): string | null {
  if (crop.ratio === 'original' || vw < 2 || vh < 2) return null
  const r = cropRect(crop, vw, vh)
  let x = even(Math.round(r.x))
  let y = even(Math.round(r.y))
  let w = even(Math.round(r.w))
  let h = even(Math.round(r.h))
  if (x + w > vw) w = even(vw - x)
  if (y + h > vh) h = even(vh - y)
  if (x < 0) x = 0
  if (y < 0) y = 0
  if (w < 2 || h < 2) return null
  const [ow, oh] = CROP_OUTPUT_SIZE[crop.ratio]
  return `crop=${w}:${h}:${x}:${y},scale=${ow}:${oh}`
}

export function cropFromPrevious(
  clips: Array<{ crop?: ClipCrop; title: string }>,
  index: number,
  ratio: CropRatio
): { crop: ClipCrop; fromTitle: string | null } {
  if (ratio === 'original') {
    return { crop: { ...DEFAULT_CROP }, fromTitle: null }
  }
  for (let i = index - 1; i >= 0; i--) {
    const prev = clips[i]?.crop
    if (prev && prev.ratio === ratio) {
      return { crop: { ...prev }, fromTitle: clips[i].title }
    }
  }
  return { crop: { ratio, cx: 0.5, cy: 0.5, zoom: 0 }, fromTitle: null }
}

export function containRect(
  containerW: number,
  containerH: number,
  videoW: number,
  videoH: number
): { x: number; y: number; w: number; h: number } {
  if (containerW <= 0 || containerH <= 0) {
    return { x: 0, y: 0, w: 0, h: 0 }
  }
  if (videoW <= 0 || videoH <= 0) {
    return { x: 0, y: 0, w: containerW, h: containerH }
  }
  const scale = Math.min(containerW / videoW, containerH / videoH)
  const w = videoW * scale
  const h = videoH * scale
  return { x: (containerW - w) / 2, y: (containerH - h) / 2, w, h }
}
