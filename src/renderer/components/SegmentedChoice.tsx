interface Choice<T extends string> {
  id: T
  label: string
  swatch?: string
  fontFamily?: string
}

interface SegmentedChoiceProps<T extends string> {
  label: string
  value: T
  options: Choice<T>[]
  onChange: (id: T) => void
  disabled?: boolean
  size?: 'md' | 'sm'
}

export default function SegmentedChoice<T extends string>({
  label,
  value,
  options,
  onChange,
  disabled = false,
  size = 'md'
}: SegmentedChoiceProps<T>): React.JSX.Element {
  return (
    <div className="option-row" role="radiogroup" aria-label={label}>
      {options.map((option) => {
        const selected = value === option.id
        return (
          <button
            key={option.id}
            type="button"
            role="radio"
            aria-checked={selected}
            disabled={disabled}
            onClick={() => onChange(option.id)}
            className={`option${size === 'sm' ? ' option-sm' : ''}${selected ? ' is-selected' : ''}`}
          >
            {option.swatch ? <span className="swatch" style={{ background: option.swatch }} /> : null}
            <span style={option.fontFamily ? { fontFamily: option.fontFamily } : undefined}>{option.label}</span>
          </button>
        )
      })}
    </div>
  )
}
