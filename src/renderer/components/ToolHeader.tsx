interface ToolHeaderProps {
  title: string
  fileName: string
  onBack: () => void
}

export default function ToolHeader({
  title,
  fileName,
  onBack
}: ToolHeaderProps): React.JSX.Element {
  return (
    <>
      <button type="button" className="home-back" onClick={onBack}>
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M10 3.5L5.5 8 10 12.5"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        Home
      </button>
      <p className="titlebar-context">
        <strong>{title}</strong>
        {fileName ? <span className="titlebar-file">{fileName}</span> : null}
      </p>
    </>
  )
}
