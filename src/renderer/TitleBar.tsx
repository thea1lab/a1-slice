import type { CSSProperties } from 'react'

let _isMac: boolean | undefined

function isMac(): boolean {
  if (_isMac === undefined) _isMac = window.api.getPlatform() === 'darwin'
  return _isMac
}

export default function TitleBar() {
  const mac = isMac()
  return (
    <div
      className="flex items-center justify-between w-full h-10 px-3 select-none shrink-0 bg-bg-base"
      style={{ WebkitAppRegion: 'drag' } as CSSProperties}
    >
      <div className={mac ? 'w-20' : 'w-0'} />

      <span className="text-xs font-medium tracking-wide text-neutral-500">
        A1 Slice
      </span>

      <div className={mac ? 'w-20' : 'w-36'} />
    </div>
  )
}
