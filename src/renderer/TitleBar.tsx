import { useState, useEffect } from 'react'

export default function TitleBar(): React.JSX.Element {
  const [isMac, setIsMac] = useState(false)

  useEffect(() => {
    setIsMac(window.api.getPlatform() === 'darwin')
  }, [])

  return (
    <div
      className="flex items-center justify-between w-full h-10 px-3 select-none shrink-0"
      style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
    >
      {/* Left spacer — on macOS traffic lights occupy this space */}
      <div className="w-20" />

      <span className="text-xs font-medium tracking-wide text-neutral-500">
        A1 Slice
      </span>

      {/* Window controls — only on Windows/Linux */}
      {!isMac && (
        <div
          className="flex items-center gap-1"
          style={{ WebkitAppRegion: 'no-drag' } as React.CSSProperties}
        >
          <button
            onClick={() => window.api.windowMinimize()}
            className="w-8 h-8 flex items-center justify-center rounded text-neutral-400 hover:bg-neutral-800 hover:text-white transition-colors"
            aria-label="Minimize"
          >
            &#x2500;
          </button>
          <button
            onClick={() => window.api.windowMaximize()}
            className="w-8 h-8 flex items-center justify-center rounded text-neutral-400 hover:bg-neutral-800 hover:text-white transition-colors"
            aria-label="Maximize"
          >
            &#x25A1;
          </button>
          <button
            onClick={() => window.api.windowClose()}
            className="w-8 h-8 flex items-center justify-center rounded text-neutral-400 hover:bg-red-900 hover:text-white transition-colors"
            aria-label="Close"
          >
            &#x2715;
          </button>
        </div>
      )}

      {isMac && <div className="w-20" />}
    </div>
  )
}
