import type {
  CaptionColor,
  CaptionFont,
  CaptionLook,
  CaptionPosition,
  CaptionProject,
  CaptionSize,
  CaptionSource,
  CaptionStyle
} from './types'

export type { CaptionColor, CaptionFont, CaptionPosition, CaptionSize, CaptionStyle }

export const DEFAULT_CAPTION_STYLE: CaptionStyle = {
  color: 'white',
  position: 'bottom',
  size: 'large',
  font: 'sans'
}

/** ASS colour is &HAABBGGRR. Alpha 00 is opaque. */
export const CAPTION_INK: Record<CaptionColor, { hex: string; ass: string; outline: string }> = {
  white: { hex: '#ffffff', ass: '&H00FFFFFF', outline: '&H00000000' },
  cream: { hex: '#fff8e0', ass: '&H00E0F8FF', outline: '&H00000000' },
  yellow: { hex: '#ffe14a', ass: '&H004AE1FF', outline: '&H00000000' },
  black: { hex: '#111111', ass: '&H00111111', outline: '&H00FFFFFF' }
}

/**
 * Sizes are CSS pixels on a 288-tall caption frame. An SRT burn uses libass PlayResY 288,
 * then scales to the video. Small and large match the older preset burns.
 * libass FontSize is the Windows cell, so the burn multiplies by the face's cell ratio.
 */
export const CAPTION_METRICS: Record<CaptionSize, { fontSize: number; margin: number }> = {
  small: { fontSize: 18, margin: 36 },
  medium: { fontSize: 24, margin: 60 },
  large: { fontSize: 28, margin: 90 }
}

/**
 * libass outline, in PlayResY 288 script pixels. The preview stroke is 1px on the player,
 * about 0.55 script pixels on a 560px-tall frame. Outline 2 fills the letters.
 */
export const CAPTION_BURN_OUTLINE = 0.55

export const CAPTION_FONT_FAMILY: Record<CaptionFont, string> = {
  sans: '"Noto Sans", "Liberation Sans", "DejaVu Sans", sans-serif',
  serif: '"Noto Serif", "Liberation Serif", "DejaVu Serif", Georgia, serif',
  mono: '"Noto Sans Mono", "Liberation Mono", "DejaVu Sans Mono", ui-monospace, monospace'
}

const COLORS = new Set<CaptionColor>(['white', 'cream', 'yellow', 'black'])
const POSITIONS = new Set<CaptionPosition>(['bottom', 'middle', 'top'])
const SIZES = new Set<CaptionSize>(['small', 'medium', 'large'])
const FONTS = new Set<CaptionFont>(['sans', 'serif', 'mono'])

/** Wider than the three presets, still a fraction of a 288-tall caption frame. */
export const MIN_CAPTION_FONT_SIZE = 8
export const MAX_CAPTION_FONT_SIZE = 96

export const CAPTION_PALETTE: { label: string; hex: string; named?: CaptionColor }[] = [
  { label: 'White', hex: CAPTION_INK.white.hex, named: 'white' },
  { label: 'Cream', hex: CAPTION_INK.cream.hex, named: 'cream' },
  { label: 'Yellow', hex: CAPTION_INK.yellow.hex, named: 'yellow' },
  { label: 'Gold', hex: '#ffb83e' },
  { label: 'Orange', hex: '#ff9a3c' },
  { label: 'Red', hex: '#ff5a4a' },
  { label: 'Pink', hex: '#ff8ad4' },
  { label: 'Green', hex: '#7dff6a' },
  { label: 'Cyan', hex: '#6aefff' },
  { label: 'Blue', hex: '#6aa6ff' },
  { label: 'Purple', hex: '#c48aff' },
  { label: 'Black', hex: CAPTION_INK.black.hex, named: 'black' }
]

function oneOf<T extends string>(value: unknown, allowed: Set<T>): T | null {
  return typeof value === 'string' && allowed.has(value as T) ? (value as T) : null
}

export function parseHexColor(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const match = value.trim().match(/^#([0-9a-fA-F]{6})$/)
  return match ? `#${match[1].toLowerCase()}` : null
}

export function clampCaptionFontSize(value: number): number {
  if (!Number.isFinite(value)) return CAPTION_METRICS.large.fontSize
  return Math.min(MAX_CAPTION_FONT_SIZE, Math.max(MIN_CAPTION_FONT_SIZE, Math.round(value)))
}

function parseFontSize(value: unknown): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value)) return null
  return clampCaptionFontSize(value)
}

export function parseCaptionStyle(value: unknown): CaptionStyle | null {
  if (!value || typeof value !== 'object') return null
  const row = value as Record<string, unknown>
  const color = oneOf(row.color, COLORS)
  const position = oneOf(row.position, POSITIONS)
  const size = oneOf(row.size, SIZES)
  const font = oneOf(row.font, FONTS)
  if (!color || !position || !size || !font) return null
  const style: CaptionStyle = { color, position, size, font }
  const customColor = parseHexColor(row.customColor)
  if (customColor) style.customColor = customColor
  const fontSize = parseFontSize(row.fontSize)
  if (fontSize !== null) {
    const preset = (Object.keys(CAPTION_METRICS) as CaptionSize[]).find(
      (key) => CAPTION_METRICS[key].fontSize === fontSize
    )
    if (preset) style.size = preset
    else style.fontSize = fontSize
  }
  return style
}

function hexToAss(hex: string): string {
  const red = hex.slice(1, 3)
  const green = hex.slice(3, 5)
  const blue = hex.slice(5, 7)
  return `&H00${blue}${green}${red}`.toUpperCase()
}

function channelLinear(value: number): number {
  const s = value / 255
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
}

function relativeLuminance(hex: string): number {
  const red = channelLinear(parseInt(hex.slice(1, 3), 16))
  const green = channelLinear(parseInt(hex.slice(3, 5), 16))
  const blue = channelLinear(parseInt(hex.slice(5, 7), 16))
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

export function captionInk(style: CaptionStyle): { hex: string; ass: string; outline: string } {
  const custom = parseHexColor(style.customColor)
  if (!custom) return CAPTION_INK[style.color]
  return {
    hex: custom,
    ass: hexToAss(custom),
    outline: relativeLuminance(custom) < 0.2 ? '&H00FFFFFF' : '&H00000000'
  }
}

export function captionMetrics(style: CaptionStyle): { fontSize: number; margin: number } {
  if (style.fontSize == null) return CAPTION_METRICS[style.size]
  const fontSize = clampCaptionFontSize(style.fontSize)
  const preset = (Object.values(CAPTION_METRICS) as { fontSize: number; margin: number }[]).find(
    (metrics) => metrics.fontSize === fontSize
  )
  if (preset) return preset
  return {
    fontSize,
    margin: Math.round((fontSize * CAPTION_METRICS.large.margin) / CAPTION_METRICS.large.fontSize)
  }
}

export function styleWithPresetSize(style: CaptionStyle, size: CaptionSize): CaptionStyle {
  const next: CaptionStyle = { ...style, size }
  delete next.fontSize
  return next
}

export function styleWithFontSize(style: CaptionStyle, px: number): CaptionStyle {
  const fontSize = clampCaptionFontSize(px)
  const preset = (Object.keys(CAPTION_METRICS) as CaptionSize[]).find(
    (key) => CAPTION_METRICS[key].fontSize === fontSize
  )
  if (preset) return styleWithPresetSize(style, preset)
  return { ...style, fontSize }
}

export function styleWithColor(style: CaptionStyle, hex: string): CaptionStyle {
  const normalized = parseHexColor(hex)
  if (!normalized) return style
  const named = CAPTION_PALETTE.find((item) => item.named && item.hex === normalized)
  if (named?.named) {
    const next: CaptionStyle = { ...style, color: named.named }
    delete next.customColor
    return next
  }
  return {
    ...style,
    color: relativeLuminance(normalized) < 0.2 ? 'black' : 'white',
    customColor: normalized
  }
}

export function coerceCaptionLook(look: unknown): CaptionLook | null {
  if (look === 'srt') return 'srt'
  if (look === 'burn' || look === 'burn-large' || look === 'burn-small') return 'burn'
  return null
}

export function coerceCaptionStyle(look: unknown, style: unknown): CaptionStyle {
  return (
    parseCaptionStyle(style) ?? {
      ...DEFAULT_CAPTION_STYLE,
      size: look === 'burn-small' ? 'small' : 'large'
    }
  )
}

export function captionForceStyle(style: CaptionStyle, fontName: string, cellRatio = 1): string {
  const ink = captionInk(style)
  const metrics = captionMetrics(style)
  const ratio = Number.isFinite(cellRatio) && cellRatio > 0 ? cellRatio : 1
  const alignment = style.position === 'top' ? 8 : style.position === 'middle' ? 5 : 2
  const margin = style.position === 'middle' ? 0 : metrics.margin
  return [
    `FontName=${fontName}`,
    `FontSize=${Math.round(metrics.fontSize * ratio)}`,
    `PrimaryColour=${ink.ass}`,
    `OutlineColour=${ink.outline}`,
    'BorderStyle=1',
    `Outline=${CAPTION_BURN_OUTLINE}`,
    'Shadow=0',
    'Bold=0',
    `Alignment=${alignment}`,
    `MarginV=${margin}`
  ].join(',')
}

export function blankCaptionProject(hasTranscript: boolean): CaptionProject {
  return {
    source: hasTranscript ? 'transcript' : 'manual',
    look: 'srt',
    style: DEFAULT_CAPTION_STYLE,
    cues: []
  }
}

export function presentCaptionProject(
  saved: CaptionProject | null,
  hasTranscript: boolean
): CaptionProject {
  if (!saved) return blankCaptionProject(hasTranscript)
  const source: CaptionSource = saved.source === 'manual' ? 'manual' : 'transcript'
  const filePath = source === 'manual' && saved.filePath?.trim() ? saved.filePath : undefined
  return {
    source,
    look: coerceCaptionLook(saved.look) ?? 'srt',
    style: coerceCaptionStyle(saved.look, saved.style),
    cues: Array.isArray(saved.cues) ? saved.cues : [],
    ...(filePath ? { filePath } : {})
  }
}
