import type { CSSProperties } from 'react'

export default function TitleBar() {
  return (
    <div
      className="relative flex items-center w-full h-10 px-3 select-none shrink-0 bg-bg-base"
      style={{ WebkitAppRegion: 'drag' } as CSSProperties}
    >
      <span className="absolute inset-x-0 text-center text-xs font-medium tracking-wide text-neutral-500 pointer-events-none">
        A1 Slice
      </span>
    </div>
  )
}
