import { useState } from 'react'
import SegmentedChoice from './SegmentedChoice'
import {
  CAPTION_FONT_FAMILY,
  CAPTION_PALETTE,
  MAX_CAPTION_FONT_SIZE,
  MIN_CAPTION_FONT_SIZE,
  captionInk,
  captionMetrics,
  styleWithColor,
  styleWithFontSize,
  styleWithPresetSize,
  type CaptionFont,
  type CaptionPosition,
  type CaptionSize,
  type CaptionStyle
} from '../../shared/captions'

const POSITIONS: { id: CaptionPosition; label: string }[] = [
  { id: 'bottom', label: 'Bottom' },
  { id: 'middle', label: 'Middle' },
  { id: 'top', label: 'Top' }
]

const SIZES: { id: CaptionSize; label: string }[] = [
  { id: 'small', label: 'Small' },
  { id: 'medium', label: 'Medium' },
  { id: 'large', label: 'Large' }
]

const FONTS: { id: CaptionFont; label: string; fontFamily: string }[] = [
  { id: 'sans', label: 'Sans', fontFamily: CAPTION_FONT_FAMILY.sans },
  { id: 'serif', label: 'Serif', fontFamily: CAPTION_FONT_FAMILY.serif },
  { id: 'mono', label: 'Mono', fontFamily: CAPTION_FONT_FAMILY.mono }
]

interface CaptionStylePickerProps {
  style: CaptionStyle
  onChange: (style: CaptionStyle) => void
}

export default function CaptionStylePicker({ style, onChange }: CaptionStylePickerProps): React.JSX.Element {
  const hex = captionInk(style).hex
  const fontSize = captionMetrics(style).fontSize
  const custom = !CAPTION_PALETTE.some((item) => item.hex === hex)
  const [sizeDraft, setSizeDraft] = useState<string | null>(null)

  return (
    <div className="grid w-full grid-cols-1 gap-4 md:grid-cols-2">
      <div className="flex flex-col items-start gap-2">
        <h3 className="section-label">Color</h3>
        <div className="color-palette" role="radiogroup" aria-label="Caption color">
          {CAPTION_PALETTE.map((item) => {
            const selected = item.hex === hex
            return (
              <button
                key={item.hex}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={item.label}
                title={item.label}
                className={`color-swatch${selected ? ' is-selected' : ''}`}
                style={{ background: item.hex }}
                onClick={() => onChange(styleWithColor(style, item.hex))}
              />
            )
          })}
          <span className="color-custom-wrap">
            <label
              className={`color-custom${custom ? ' is-selected' : ''}`}
              title="Custom color"
              style={custom ? { background: hex } : undefined}
            >
              <input
                type="color"
                aria-label="Custom color"
                value={hex}
                onChange={(event) => onChange(styleWithColor(style, event.target.value))}
              />
            </label>
            Custom
          </span>
        </div>
      </div>
      <div className="flex flex-col items-start gap-2">
        <h3 className="section-label">Position</h3>
        <SegmentedChoice
          label="Caption position"
          value={style.position}
          options={POSITIONS}
          onChange={(position) => onChange({ ...style, position })}
        />
      </div>
      <div className="flex flex-col items-start gap-2">
        <h3 className="section-label">Size</h3>
        <div className="flex flex-wrap items-center gap-2">
          <SegmentedChoice
            label="Caption size"
            value={style.fontSize == null ? style.size : ('' as CaptionSize)}
            options={SIZES}
            onChange={(size) => onChange(styleWithPresetSize(style, size))}
          />
          <label className="font-size-label">
            Font size
            <input
              type="number"
              className="field font-size-field"
              aria-label="Font size"
              min={MIN_CAPTION_FONT_SIZE}
              max={MAX_CAPTION_FONT_SIZE}
              value={sizeDraft ?? String(fontSize)}
              onFocus={() => setSizeDraft(String(fontSize))}
              onBlur={() => {
                const next = Number(sizeDraft)
                if (sizeDraft !== null && sizeDraft.trim() !== '' && Number.isFinite(next)) {
                  onChange(styleWithFontSize(style, next))
                }
                setSizeDraft(null)
              }}
              onChange={(event) => {
                setSizeDraft(event.target.value)
                const next = Number(event.target.value)
                if (event.target.value.trim() !== '' && Number.isFinite(next)) {
                  onChange(styleWithFontSize(style, next))
                }
              }}
            />
          </label>
        </div>
      </div>
      <div className="flex flex-col items-start gap-2">
        <h3 className="section-label">Font</h3>
        <SegmentedChoice
          label="Caption font"
          value={style.font}
          options={FONTS}
          onChange={(font) => onChange({ ...style, font })}
        />
      </div>
    </div>
  )
}
