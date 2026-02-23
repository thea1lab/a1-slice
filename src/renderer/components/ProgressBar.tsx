interface ProgressBarProps {
  percent: number
  label?: string
  sublabel?: string
}

export default function ProgressBar({
  percent,
  label,
  sublabel
}: ProgressBarProps): React.JSX.Element {
  return (
    <div className="w-full space-y-2">
      {(label || sublabel) && (
        <div className="flex items-center justify-between text-sm">
          {label && (
            <span className="text-neutral-300 font-medium">{label}</span>
          )}
          {sublabel && (
            <span className="text-neutral-500 tabular-nums">{sublabel}</span>
          )}
        </div>
      )}
      <div className="w-full h-2 bg-bg-input rounded-full overflow-hidden">
        <div
          className="h-full bg-accent rounded-full transition-all duration-500 ease-out"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  )
}
