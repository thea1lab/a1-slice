import { describe, expect, it } from 'vitest'
import {
  CAPTION_BURN_OUTLINE,
  CAPTION_INK,
  CAPTION_METRICS,
  captionForceStyle,
  captionInk,
  coerceCaptionStyle,
  DEFAULT_CAPTION_STYLE,
  parseCaptionStyle,
  styleWithColor,
  styleWithFontSize
} from '../captions'

describe('caption style', () => {
  it('builds an ASS force style for colour, place, size, and font name', () => {
    const force = captionForceStyle(
      { color: 'yellow', position: 'top', size: 'medium', font: 'serif' },
      'Noto Serif'
    )
    expect(force).toContain('FontName=Noto Serif')
    expect(force).toContain(`FontSize=${CAPTION_METRICS.medium.fontSize}`)
    expect(force).toContain(`PrimaryColour=${CAPTION_INK.yellow.ass}`)
    expect(force).toContain('Alignment=8')
    expect(force).toContain(`MarginV=${CAPTION_METRICS.medium.margin}`)
    expect(force).toContain(`Outline=${CAPTION_BURN_OUTLINE}`)
    expect(force).toContain('Shadow=0')
  })

  it('centers a caption and drops the edge margin', () => {
    expect(captionForceStyle({ ...DEFAULT_CAPTION_STYLE, position: 'middle' }, 'Noto Sans')).toContain(
      'Alignment=5'
    )
    expect(captionForceStyle({ ...DEFAULT_CAPTION_STYLE, position: 'middle' }, 'Noto Sans')).toContain(
      'MarginV=0'
    )
  })

  it('outlines black type in white', () => {
    const force = captionForceStyle({ ...DEFAULT_CAPTION_STYLE, color: 'black' }, 'Noto Sans')
    expect(force).toContain(`PrimaryColour=${CAPTION_INK.black.ass}`)
    expect(force).toContain(`OutlineColour=${CAPTION_INK.black.outline}`)
  })

  it('keeps the older large and small sizes', () => {
    expect(CAPTION_METRICS.large.fontSize).toBe(28)
    expect(CAPTION_METRICS.large.margin).toBe(90)
    expect(CAPTION_METRICS.small.fontSize).toBe(18)
    expect(CAPTION_METRICS.small.margin).toBe(36)
  })

  it('rejects a partial style', () => {
    expect(parseCaptionStyle({ color: 'pink', position: 'top', size: 'large', font: 'sans' })).toBeNull()
    expect(parseCaptionStyle(null)).toBeNull()
  })

  it('burns a custom colour and a custom font size', () => {
    const style = {
      ...DEFAULT_CAPTION_STYLE,
      customColor: '#ff5a4a',
      fontSize: 40
    }
    const force = captionForceStyle(style, 'Noto Sans')
    expect(force).toContain('FontSize=40')
    expect(force).toContain(`PrimaryColour=${captionInk(style).ass}`)
    expect(force).toContain('PrimaryColour=&H004A5AFF')
    expect(force).toContain('OutlineColour=&H00000000')
    expect(force).toContain('MarginV=129')
  })

  it('scales the burn to the font cell and leaves the margin alone', () => {
    const style = { ...DEFAULT_CAPTION_STYLE, fontSize: 13, font: 'mono' as const }
    const force = captionForceStyle(style, 'Noto Sans Mono Medium', 1.618)
    expect(force).toContain('FontSize=21')
    expect(force).toContain('MarginV=42')
    expect(force).toContain('FontName=Noto Sans Mono Medium')
    expect(captionForceStyle(style, 'Noto Sans Mono', Number.NaN)).toContain('FontSize=13')
  })

  it('outlines a dark custom colour in white', () => {
    const style = { ...DEFAULT_CAPTION_STYLE, customColor: '#112233' }
    expect(captionForceStyle(style, 'Noto Sans')).toContain('OutlineColour=&H00FFFFFF')
    expect(captionForceStyle(style, 'Noto Sans')).toContain('PrimaryColour=&H00332211')
  })

  it('stores a palette colour by name and a typed size by preset', () => {
    expect(styleWithColor(DEFAULT_CAPTION_STYLE, '#ffe14a')).toEqual({
      ...DEFAULT_CAPTION_STYLE,
      color: 'yellow'
    })
    expect(styleWithColor(DEFAULT_CAPTION_STYLE, '#ff5a4a').customColor).toBe('#ff5a4a')
    expect(styleWithFontSize(DEFAULT_CAPTION_STYLE, 28)).toEqual({
      ...DEFAULT_CAPTION_STYLE,
      size: 'large'
    })
    expect(styleWithFontSize(DEFAULT_CAPTION_STYLE, 40).fontSize).toBe(40)
    expect(styleWithFontSize(DEFAULT_CAPTION_STYLE, 4).fontSize).toBe(8)
    expect(styleWithFontSize(DEFAULT_CAPTION_STYLE, 200).fontSize).toBe(96)
  })

  it('reads a custom colour and size, and keeps a named style unchanged', () => {
    expect(
      parseCaptionStyle({
        color: 'white',
        customColor: '#FF5A4A',
        position: 'bottom',
        size: 'large',
        fontSize: 40,
        font: 'sans'
      })
    ).toEqual({
      ...DEFAULT_CAPTION_STYLE,
      customColor: '#ff5a4a',
      fontSize: 40
    })
    expect(
      parseCaptionStyle({ color: 'white', position: 'bottom', size: 'large', font: 'sans', fontSize: 18 })
    ).toEqual({ ...DEFAULT_CAPTION_STYLE, size: 'small' })
  })

  it('reads a complete style and falls back when it is missing', () => {
    expect(
      parseCaptionStyle({ color: 'cream', position: 'bottom', size: 'small', font: 'mono' })
    ).toEqual({ color: 'cream', position: 'bottom', size: 'small', font: 'mono' })
    expect(coerceCaptionStyle('burn-small', { color: 'nope' })).toEqual({
      ...DEFAULT_CAPTION_STYLE,
      size: 'small'
    })
    expect(coerceCaptionStyle('burn-large', undefined)).toEqual(DEFAULT_CAPTION_STYLE)
  })
})
