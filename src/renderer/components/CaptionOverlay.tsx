import type { CaptionLook } from '../../shared/types'

interface CaptionOverlayProps {
  text: string
  look: CaptionLook
}

export default function CaptionOverlay({ text, look }: CaptionOverlayProps): React.JSX.Element | null {
  if (!text) return null
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-8 flex justify-center px-8">
      <p
        className={`max-w-xl text-center text-white ${
          look === 'burn-small' ? 'text-sm' : 'text-2xl font-semibold'
        } ${look === 'srt' ? 'opacity-70' : ''}`}
        style={{ textShadow: '0 1px 2px rgba(0,0,0,0.9), 0 0 8px rgba(0,0,0,0.8)' }}
      >
        {text}
      </p>
    </div>
  )
}
