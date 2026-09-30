import { useEffect, useRef, useState } from 'react'
import {
  applyEditedCaptionText,
  breakCaptionLines,
  captionFileLines,
  captionFileText,
  clampWordsPerLine,
  compactDiff,
  diffCaptionLines,
  diffSummary
} from '../../shared/captionEdit'
import type { TranscriptSegment } from '../../shared/types'

interface CaptionAgentInfo {
  id: string
  label: string
}

interface FixWordsScreenProps {
  fileName: string
  lines: TranscriptSegment[]
  onBack: () => void
  onSave: (lines: TranscriptSegment[]) => Promise<string | null>
}

function normalizeFile(text: string): string {
  return text.replace(/\r\n/g, '\n').trim()
}

function rememberLog(current: string[], line: string): string[] {
  if (current.at(-1) === line) return current
  return [...current, line]
}

function readWordsPerLine(value: string): number {
  if (value.trim() === '') return 8
  return clampWordsPerLine(Number(value))
}

function fileLines(text: string): string[] {
  return text.split('\n').filter((line) => line.length > 0)
}

export default function FixWordsScreen({
  fileName,
  lines,
  onBack,
  onSave
}: FixWordsScreenProps): React.JSX.Element {
  const [fileText, setFileText] = useState(() => captionFileText(lines))
  const [agents, setAgents] = useState<CaptionAgentInfo[] | null>(null)
  const [agentId, setAgentId] = useState<string | null>(null)
  const [fixTypos, setFixTypos] = useState(true)
  const [breakLines, setBreakLines] = useState(true)
  const [wordsPerLine, setWordsPerLine] = useState('8')
  const [note, setNote] = useState('')
  const [logs, setLogs] = useState<string[]>([])
  const [running, setRunning] = useState(false)
  const [agentFile, setAgentFile] = useState<string | null>(null)
  const [showChanges, setShowChanges] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const logRef = useRef<HTMLDivElement>(null)
  const alive = useRef(true)

  useEffect(() => {
    alive.current = true
    return () => {
      alive.current = false
      window.api.cancelCaptionFix()
    }
  }, [])

  useEffect(() => {
    let active = true
    window.api
      .listCaptionAgents()
      .then((found) => {
        if (!active) return
        setAgents(found)
        setAgentId(found[0]?.id ?? null)
      })
      .catch(() => {
        if (active) setAgents([])
      })
    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    const box = logRef.current
    if (box) box.scrollTop = box.scrollHeight
  }, [logs])

  useEffect(() => {
    return window.api.onCaptionFixLog((line) => {
      if (alive.current) setLogs((current) => rememberLog(current, line))
    })
  }, [])

  const dirty = normalizeFile(fileText) !== normalizeFile(captionFileText(lines))
  const typedAfter = agentFile !== null && normalizeFile(fileText) !== normalizeFile(agentFile)
  const wantsAgent = fixTypos || note.trim().length > 0
  const canAsk = breakLines || (wantsAgent && Boolean(agentId))
  const agentLabel = agents?.find((agent) => agent.id === agentId)?.label ?? 'the agent'

  const showDiff = (nextFile: string, summary?: string): string => {
    setAgentFile(nextFile)
    setFileText(nextFile)
    if (summary) return summary
    const rows = diffCaptionLines(fileLines(captionFileText(lines)), fileLines(nextFile))
    return diffSummary(rows)
  }

  const originalLines = fileLines(captionFileText(lines))
  const changedRows = diffCaptionLines(originalLines, fileLines(fileText))
  const visibleChanges = compactDiff(changedRows)
  const hasLineChanges = changedRows.some((row) => row.kind === 'add' || row.kind === 'remove')

  const ask = (): void => {
    if (running || !canAsk) return
    const current = applyEditedCaptionText(lines, fileText)
    if ('error' in current) {
      setLogs([current.error])
      return
    }
    const limit = readWordsPerLine(wordsPerLine)
    const beforeFile = captionFileText(current.segments)
    let working = current.segments
    const notes: string[] = []
    if (breakLines) {
      const tooLong = working.filter((segment) => segment.text.trim().split(/\s+/).filter(Boolean).length > limit).length
      working = breakCaptionLines(working, limit)
      const nextFile = captionFileText(working)
      if (tooLong > 0 && normalizeFile(nextFile) !== normalizeFile(beforeFile)) {
        const sentence = tooLong === 1 ? `Split 1 line longer than ${limit} words.` : `Split ${tooLong} lines longer than ${limit} words.`
        notes.push(showDiff(nextFile, sentence))
      } else {
        notes.push(`Every line is already ${limit} words or shorter.`)
      }
    }
    setLogs(notes)
    setSaveError(null)
    if (!wantsAgent) return
    if (agents === null) {
      setLogs((currentLogs) => rememberLog(currentLogs, 'Looking for an agent on this computer…'))
      return
    }
    if (!agentId) {
      setLogs((currentLogs) => rememberLog(currentLogs, 'This computer has no agent to check typos. The lines on the left are ready to save.'))
      return
    }
    const sentFile = captionFileText(working)
    setRunning(true)
    void window.api
      .fixCaptions(agentId, working, {
        fixTypos,
        breakLines: false,
        wordsPerLine: limit,
        note: note.trim()
      })
      .then((result) => {
        if (!alive.current) return
        if (result.segments) {
          const nextFile = captionFileText(result.segments)
          if (normalizeFile(nextFile) === normalizeFile(sentFile)) {
            setLogs((currentLogs) => rememberLog(currentLogs, 'No typos to fix.'))
            return
          }
          const longest = Math.max(
            0,
            ...captionFileLines(nextFile).map((line) => line.split(/\s+/).filter(Boolean).length)
          )
          if (breakLines && longest > limit) {
            setLogs((currentLogs) => rememberLog(currentLogs, 'The typo check tried to join lines. The shorter lines stayed.'))
            return
          }
          const summary = showDiff(nextFile)
          setLogs((currentLogs) => rememberLog(currentLogs, summary))
          return
        }
        const message = notes.some((line) => line.startsWith('Split '))
          ? 'The shorter lines are on the left. The typo check did not return anything else.'
          : result.error || 'The agent did not return the edited lines. The lines were left as they are.'
        setLogs((currentLogs) => rememberLog(currentLogs, message))
      })
      .catch(() => {
        if (alive.current) setLogs((currentLogs) => rememberLog(currentLogs, 'The agent could not be reached.'))
      })
      .finally(() => {
        if (alive.current) setRunning(false)
      })
  }

  const save = (): void => {
    const parsed = applyEditedCaptionText(lines, fileText)
    if ('error' in parsed) {
      setSaveError(parsed.error)
      return
    }
    if (saving) return
    setSaving(true)
    setSaveError(null)
    void onSave(parsed.segments)
      .then((error) => {
        if (error) setSaveError(error)
      })
      .catch(() => setSaveError('Could not save the transcript.'))
      .finally(() => setSaving(false))
  }

  return (
    <div className="fix-layout">
      <section className="fix-pane" aria-label={fileName}>
        <h2 className="section-label">{fileName}</h2>
        <p className="help">
          {running ? `${agentLabel} is editing this file.` : 'This is the transcript file. Type in it to change a line.'}
        </p>
        <label className="fix-compare">
          <input
            type="checkbox"
            checked={showChanges}
            onChange={(event) => setShowChanges(event.target.checked)}
          />
          Show what changed
        </label>
        {showChanges && (
          <div className="fix-diff">
            <p className="help">
              {hasLineChanges ? diffSummary(changedRows) : 'These lines match the original.'}
            </p>
            {hasLineChanges && (
              <>
                <p className="help">A minus is the original line. A plus is the fixed line.</p>
                <div className="diff custom-scrollbar" aria-label="What changed">
                  {visibleChanges.map((row, index) =>
                    row.kind === 'gap' ? (
                      <p key={`gap-${index}`} className="diff-line diff-gap">
                        …
                      </p>
                    ) : (
                      <p key={`${row.kind}-${index}`} className={`diff-line diff-${row.kind}`}>
                        <span>{row.kind === 'add' ? '+' : row.kind === 'remove' ? '-' : ' '}</span>
                        <span>{row.text}</span>
                      </p>
                    )
                  )}
                </div>
              </>
            )}
            {typedAfter && (
              <p className="help">You changed the lines again after the fix. Save keeps the lines in the file.</p>
            )}
          </div>
        )}
        <textarea
          className="field fix-file custom-scrollbar"
          value={fileText}
          disabled={running}
          spellCheck={false}
          aria-label={fileName}
          onChange={(event) => setFileText(event.target.value)}
        />
      </section>

      <section className="fix-pane custom-scrollbar" aria-label="Ask an agent">
        <h2 className="section-label">Ask an agent</h2>
        <p className="help">Long lines are split in this file. The agent checks for typos.</p>

        <div className="fix-controls custom-scrollbar">
          <h3 className="fix-label">Which agent</h3>
          {agents === null && <p className="help">Looking for agents on this computer…</p>}
          {agents !== null && agents.length === 0 && (
            <p className="help">This computer has no agent to run. You can still type in the lines and save.</p>
          )}
          {agents !== null && agents.length > 0 && (
            <div className="option-row" role="radiogroup" aria-label="Which agent">
              {agents.map((agent) => (
                <button
                  key={agent.id}
                  type="button"
                  role="radio"
                  aria-checked={agentId === agent.id}
                  className={`option${agentId === agent.id ? ' is-selected' : ''}`}
                  onClick={() => setAgentId(agent.id)}
                  disabled={running}
                >
                  {agent.label}
                </button>
              ))}
            </div>
          )}

          <h3 className="fix-label">What should it fix?</h3>
          <div className="option-row">
            <button
              type="button"
              className={`option${fixTypos ? ' is-selected' : ''}`}
              aria-pressed={fixTypos}
              disabled={running}
              onClick={() => setFixTypos((value) => !value)}
            >
              Fix typos
            </button>
            <button
              type="button"
              className={`option${breakLines ? ' is-selected' : ''}`}
              aria-pressed={breakLines}
              disabled={running}
              onClick={() => setBreakLines((value) => !value)}
            >
              Break long lines
            </button>
          </div>
          {breakLines && (
            <label className="fix-words">
              <span>Words on a line</span>
              <input
                className="field"
                inputMode="numeric"
                min={2}
                max={24}
                value={wordsPerLine}
                disabled={running}
                onChange={(event) => {
                  const next = event.target.value
                  if (next === '' || /^\d{1,2}$/.test(next)) setWordsPerLine(next)
                }}
                onBlur={() => setWordsPerLine(String(readWordsPerLine(wordsPerLine)))}
              />
            </label>
          )}

          <label className="fix-note">
            <span className="fix-label">Anything else</span>
            <textarea
              className="field"
              rows={2}
              value={note}
              disabled={running}
              placeholder="For example: the name is Anna, not Ana"
              onChange={(event) => setNote(event.target.value)}
            />
          </label>

        </div>

        <div className="fix-actions">
          {running ? (
            <button type="button" className="btn btn-secondary" onClick={() => window.api.cancelCaptionFix()}>
              Stop
            </button>
          ) : (
            <button type="button" className={`btn ${dirty ? 'btn-secondary' : 'btn-primary'}`} disabled={!canAsk} onClick={ask}>
              Fix the words
            </button>
          )}
        </div>

        <h3 className="fix-label">
          {running ? `What ${agentLabel} is doing` : logs.length > 0 ? 'What the agent did' : 'What the agent is doing'}
        </h3>
        <div className="fix-log custom-scrollbar" ref={logRef} aria-live="polite" aria-busy={running}>
          {logs.length === 0 && <p>Nothing yet. Press Fix the words and the notes will show up here.</p>}
          {logs.map((line, index) => (
            <p key={`${index}-${line}`}>{line}</p>
          ))}
        </div>

        {saveError && <p className="text-base text-red-300">{saveError}</p>}
        <div className="fix-actions">
          <button
            type="button"
            className={`btn ${dirty ? 'btn-primary' : 'btn-secondary'}`}
            disabled={saving || captionFileLines(fileText).length === 0}
            onClick={save}
          >
            {saving ? 'Saving…' : 'Save transcription'}
          </button>
          <button type="button" className="btn btn-secondary" disabled={saving} onClick={onBack}>
            {dirty ? 'Back without saving' : 'Back to captions'}
          </button>
        </div>
        <p className="help">Save writes the transcript next to the video.</p>
      </section>
    </div>
  )
}
