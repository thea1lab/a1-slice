import type { ToolId } from '../../shared/types'

interface ToolIconProps {
  id: ToolId
  className?: string
}

export default function ToolIcon({ id, className }: ToolIconProps): React.JSX.Element {
  const common = {
    viewBox: '0 0 32 32',
    fill: 'none',
    className,
    'aria-hidden': true as const
  }
  if (id === 'transcribe') {
    return (
      <svg {...common}>
        <rect x="12" y="3" width="8" height="14" rx="4" stroke="currentColor" strokeWidth="1.8" />
        <path d="M8.5 14.5a7.5 7.5 0 0 0 15 0" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <path d="M16 22v4.5M11.5 26.5h9" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    )
  }
  if (id === 'find') {
    return (
      <svg {...common}>
        <path d="M6 9.5 9.2 6h13.6L26 9.5" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" />
        <path d="M6 9.5h20" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
        <rect x="6" y="9.5" width="20" height="14" rx="1.5" stroke="currentColor" strokeWidth="1.8" />
        <path d="M12 16.5h8" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
      </svg>
    )
  }
  if (id === 'reframe') {
    return (
      <svg {...common}>
        <rect x="4" y="8" width="18" height="16" rx="1.5" stroke="currentColor" strokeWidth="1.8" />
        <rect x="14" y="5" width="14" height="18" rx="1.5" stroke="currentColor" strokeWidth="1.8" />
      </svg>
    )
  }
  return (
    <svg {...common}>
      <rect x="4" y="6" width="24" height="16" rx="2" stroke="currentColor" strokeWidth="1.8" />
      <path d="M8 16.5h10M8 20h16" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    </svg>
  )
}
