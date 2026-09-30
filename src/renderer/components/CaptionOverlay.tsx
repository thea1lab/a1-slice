import { useEffect, useRef, useState } from 'react'
import {
  CAPTION_FONT_FAMILY,
  captionInk,
  captionMetrics,
  type CaptionStyle
} from '../../shared/captions'

interface CaptionOverlayProps {
  text: string
  style: CaptionStyle
}

function outlineShadow(light: boolean): string {
  const ink = light ? 'rgba(255,255,255,0.95)' : 'rgba(0,0,0,0.92)'
  return [
    `-1px -1px 0 ${ink}`,
    `1px -1px 0 ${ink}`,
    `-1px 1px 0 ${ink}`,
    `1px 1px 0 ${ink}`
  ].join(', ')
}

export default function CaptionOverlay({ text, style }: CaptionOverlayProps): React.JSX.Element | null {
  const frame = useRef<HTMLDivElement>(null)
  const [height, setHeight] = useState(0)

  useEffect(() => {
    const el = frame.current
    if (!el) return
    const measure = (): void => setHeight(el.clientHeight)
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  }, [text])

  if (!text) return null

  const box = height || 320
  const metrics = captionMetrics(style)
  const fontSize = (metrics.fontSize / 288) * box
  const margin = (metrics.margin / 288) * box
  const ink = captionInk(style)

  return (
    <div
      ref={frame}
      className="pointer-events-none absolute inset-0 flex justify-center px-8"
      style={{
        alignItems: style.position === 'top' ? 'flex-start' : style.position === 'middle' ? 'center' : 'flex-end',
        paddingTop: style.position === 'top' ? margin : 12,
        paddingBottom: style.position === 'bottom' ? margin : 12
      }}
    >
      <p
        className="max-w-[86%] text-center"
        style={{
          color: ink.hex,
          fontFamily: CAPTION_FONT_FAMILY[style.font],
          fontSize,
          fontWeight: 500,
          lineHeight: 1.2,
          textShadow: outlineShadow(ink.outline === '&H00FFFFFF')
        }}
      >
        {text}
      </p>
    </div>
  )
}
