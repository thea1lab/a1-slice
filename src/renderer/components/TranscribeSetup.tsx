import type { VideoLanguage } from '../../shared/types'
import { fileNameOf } from '../lib/fileName'
import SegmentedChoice from './SegmentedChoice'

const LANGUAGES: { id: VideoLanguage; label: string }[] = [
  { id: 'auto', label: 'Auto-detect' },
  { id: 'en', label: 'English' },
  { id: 'pt', label: 'Portuguese' },
  { id: 'es', label: 'Spanish' }
]

interface TranscribeSetupProps {
  videoPath: string
  language: VideoLanguage
  entropyThold: number
  maxContext: number
  beamSize: number
  temperatureInc: number
  savedCount: number
  continueNote: string | null
  onLanguageChange: (language: VideoLanguage) => void
  onEntropyTholdChange: (value: number) => void
  onMaxContextChange: (value: number) => void
  onBeamSizeChange: (value: number) => void
  onTemperatureIncChange: (value: number) => void
  onStart: () => void
  onUseSaved: () => void
}

export default function TranscribeSetup({
  videoPath,
  language,
  entropyThold,
  maxContext,
  beamSize,
  temperatureInc,
  savedCount,
  continueNote,
  onLanguageChange,
  onEntropyTholdChange,
  onMaxContextChange,
  onBeamSizeChange,
  onTemperatureIncChange,
  onStart,
  onUseSaved
}: TranscribeSetupProps): React.JSX.Element {
  const fileName = fileNameOf(videoPath)

  return (
    <div className="sheet">
      <div>
        <h1 className="display">Transcribe</h1>
        <p className="lead mt-3">{fileName}</p>
        <p className="lead mt-2">
          The speech stays on this computer.{continueNote ? ` ${continueNote}` : ''}
        </p>
      </div>

      <section className="block">
        <h2 className="section-label">Spoken language</h2>
        <p className="help">
          Auto-detect is enough when the speech is clear. Pick a language when that guess is
          often wrong, such as a Portuguese video that comes out in English.
        </p>
        <SegmentedChoice
          label="Spoken language"
          value={language}
          options={LANGUAGES}
          onChange={onLanguageChange}
        />
      </section>

      {savedCount > 0 ? (
        <section className="notice">
          <p>
            This video already has a transcript ({savedCount}{' '}
            {savedCount === 1 ? 'line' : 'lines'}). Use that, or write it again with the language
            above.
          </p>
          <div className="actions">
            <button type="button" onClick={onUseSaved} className="btn btn-primary btn-lg">
              Use the saved transcript
            </button>
            <button type="button" onClick={onStart} className="btn btn-secondary btn-lg">
              Transcribe again
            </button>
          </div>
        </section>
      ) : (
        <button type="button" onClick={onStart} className="btn btn-primary btn-lg">
          Start transcribing
        </button>
      )}

      <details className="tune">
        <summary>If the words come out wrong</summary>
        <div className="tune-body">
          <p className="help">
            Leave these as they are unless a transcript is missing words, full of junk, or stuck
            repeating itself.
          </p>

          <div className="setting">
            <h3>Drop uncertain words</h3>
            <p>
              Each guess gets a score for how unsure it is. That score is called entropy. Above
              this number, the guess is thrown out and tried again. 2.8 is the usual value. Raise
              it when real words are missing. Lower it when the transcript invents words or
              repeats the same phrase.
            </p>
            <input
              id="entropy-thold"
              aria-label="Drop uncertain words"
              type="number"
              step="0.1"
              min="0"
              max="10"
              value={entropyThold}
              onChange={(e) => {
                const value = parseFloat(e.target.value)
                if (!isNaN(value)) onEntropyTholdChange(value)
              }}
              className="field"
            />
          </div>

          <div className="setting">
            <h3>Words it remembers</h3>
            <p>
              How many earlier words it keeps in mind while it listens. That memory is called
              context. More of it keeps names and the topic steady. Too much makes it repeat
              itself or stall. 64 is the usual value. 0 means it does not look back.
            </p>
            <input
              id="max-context"
              aria-label="Words it remembers"
              type="number"
              step="1"
              min="-1"
              max="1024"
              value={maxContext}
              onChange={(e) => {
                const value = parseInt(e.target.value, 10)
                if (!isNaN(value)) onMaxContextChange(value)
              }}
              className="field"
            />
          </div>

          <div className="setting">
            <h3>How hard it searches</h3>
            <p>
              How many ways of hearing a phrase it compares before it chooses. Higher is slower
              and sometimes more accurate. 5 is the usual value.
            </p>
            <input
              id="beam-size"
              aria-label="How hard it searches"
              type="number"
              step="1"
              min="-1"
              max="16"
              value={beamSize}
              onChange={(e) => {
                const value = parseInt(e.target.value, 10)
                if (!isNaN(value)) onBeamSizeChange(value)
              }}
              className="field"
            />
          </div>

          <div className="setting">
            <h3>How loosely it retries</h3>
            <p>
              When a passage is thrown out, the next try is a little looser. This is the size of
              that step. 0.1 is a small step. 0 means it tries only once.
            </p>
            <input
              id="temperature-inc"
              aria-label="How loosely it retries"
              type="number"
              step="0.05"
              min="0"
              max="1"
              value={temperatureInc}
              onChange={(e) => {
                const value = parseFloat(e.target.value)
                if (!isNaN(value)) onTemperatureIncChange(value)
              }}
              className="field"
            />
          </div>
        </div>
      </details>
    </div>
  )
}
