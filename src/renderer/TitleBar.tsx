import type { CSSProperties, ReactNode } from 'react'

export default function TitleBar({ children }: { children?: ReactNode }) {
  const platform = typeof window !== 'undefined' ? window.api.getPlatform() : 'linux'
  const chromePad: CSSProperties =
    platform === 'darwin'
      ? { paddingLeft: 78 }
      : platform === 'win32'
        ? { paddingRight: 140 }
        : {}

  return (
    <div
      className="flex items-center w-full h-12 px-3 gap-4 select-none shrink-0 bg-bg-base border-b border-white/8"
      style={{ WebkitAppRegion: 'drag', ...chromePad } as CSSProperties}
    >
      <span className="text-xs tracking-wide text-neutral-500 shrink-0">A1 Slice</span>
      {children && (
        <div
          className="flex-1 min-w-0 flex justify-center"
          style={{ WebkitAppRegion: 'no-drag' } as CSSProperties}
        >
          {children}
        </div>
      )}
    </div>
  )
}
