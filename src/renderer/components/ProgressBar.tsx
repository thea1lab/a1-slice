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
        <div className="flex items-center justify-between gap-4 text-base">
          {label && (
            <span className="text-neutral-300 font-medium">{label}</span>
          )}
          {sublabel && (
            <span className="text-neutral-500 tabular-nums">{sublabel}</span>
          )}
        </div>
      )}
      <div className="w-full h-[3px] bg-[#2c2c2c] overflow-hidden">
        <div
          className="h-full bg-accent transition-all duration-500 ease-out"
          style={{ width: `${percent}%` }}
        />
      </div>
    </div>
  )
}
