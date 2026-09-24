import type { ToolId } from '../../shared/types'
import ToolIcon from '../components/ToolIcon'

const TOOLS: { id: ToolId; title: string; detail: string }[] = [
  {
    id: 'transcribe',
    title: 'Transcribe',
    detail: 'Turn speech into a saved transcript.'
  },
  {
    id: 'find',
    title: 'Find best parts',
    detail: 'Keep the moments you want, then export them.'
  },
  {
    id: 'reframe',
    title: 'Reframe',
    detail: 'Fit the picture to Story, YouTube, or square.'
  },
  {
    id: 'captions',
    title: 'Captions',
    detail: 'Save a subtitle file, or put the words on the video.'
  }
]

interface HomeScreenProps {
  onOpen: (tool: ToolId) => void
}

export default function HomeScreen({ onOpen }: HomeScreenProps): React.JSX.Element {
  return (
    <div className="home">
      <header className="home-head">
        <p className="eyebrow">A1 Slice</p>
        <h1 className="display">What do you want to do?</h1>
        <p className="lead mt-3">Pick a tool. You’ll choose the video on the next screen.</p>
      </header>
      <div className="home-grid">
        {TOOLS.map((tool) => (
          <button key={tool.id} type="button" className="home-tile" onClick={() => onOpen(tool.id)}>
            <span className="icon-tile">
              <ToolIcon id={tool.id} />
            </span>
            <span>
              <h2>{tool.title}</h2>
              <p className="mt-1">{tool.detail}</p>
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}
