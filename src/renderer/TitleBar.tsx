import type { CSSProperties, ReactNode } from 'react'

export default function TitleBar({ children }: { children?: ReactNode }) {
  const platform = typeof window !== 'undefined' ? window.api.getPlatform() : 'linux'
  const chromePad: CSSProperties =
    platform === 'darwin' ? { paddingLeft: 78 } : { paddingRight: 144 }

  return (
    <div className="titlebar" style={{ WebkitAppRegion: 'drag', ...chromePad } as CSSProperties}>
      <div className="titlebar-main" style={{ WebkitAppRegion: 'no-drag' } as CSSProperties}>
        {children ? children : <span className="titlebar-brand">A1 Slice</span>}
      </div>
    </div>
  )
}
