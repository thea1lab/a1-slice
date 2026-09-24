interface ChoiceCardProps {
  title: string
  detail: string
  selected?: boolean
  disabled?: boolean
  onClick: () => void
}

export default function ChoiceCard({
  title,
  detail,
  selected = false,
  disabled = false,
  onClick
}: ChoiceCardProps): React.JSX.Element {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={`choice ${selected ? 'is-selected' : ''}`}
    >
      <strong>{title}</strong>
      <em>{detail}</em>
    </button>
  )
}
