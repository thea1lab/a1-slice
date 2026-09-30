import { execFileSync, spawn, type ChildProcess } from 'child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import {
  agentOutputText,
  applyEditedCaptionText,
  captionEditPrompt,
  captionFileText,
  captionLogLine,
  clampWordsPerLine,
  emptyAgentLogState,
  extractPrintedCaption,
  isNoneAnswer,
  type AgentLogState,
  type CaptionEditRequest
} from '../shared/captionEdit'
import type { TranscriptSegment } from '../shared/types'

export interface CaptionAgentInfo {
  id: CaptionAgentId
  label: string
}

export type CaptionAgentId = 'grok' | 'claude' | 'sol' | 'agy'

export const CAPTION_AGENTS: { id: CaptionAgentId; label: string; command: string }[] = [
  { id: 'grok', label: 'Grok 4.7', command: 'grok' },
  { id: 'claude', label: 'Sonnet 5.5', command: 'claude' },
  { id: 'sol', label: 'Sol 5.6', command: 'codex' },
  { id: 'agy', label: 'Gemini 3.8 Flash', command: 'agy' }
]

export interface AgentLaunch {
  command: string
  args: string[]
  stdin: string | null
  streamJson: boolean
}

export function listInstalledAgents(
  lookup: (command: string) => string | null = commandOnPath
): CaptionAgentInfo[] {
  return CAPTION_AGENTS.filter((agent) => lookup(agent.command)).map(({ id, label }) => ({ id, label }))
}

export function commandOnPath(command: string): string | null {
  const finder = process.platform === 'win32' ? 'where' : 'which'
  try {
    const output = execFileSync(finder, [command], { encoding: 'utf8', timeout: 2000 })
    const line = output.split(/\r?\n/).map((item) => item.trim()).find(Boolean)
    return line ?? null
  } catch {
    return null
  }
}

export function agentLaunch(id: CaptionAgentId, prompt: string, workDir: string): AgentLaunch {
  const instructions = join(workDir, 'instructions.md')
  if (id === 'grok') {
    // One turn prints the transcript. Plan mode only describes the edit.
    const args = [
      '--no-plan',
      '--model',
      'grok-4.7',
      '--reasoning-effort',
      'low',
      '--output-format',
      'plain',
      '--no-subagents',
      '--disable-web-search',
      '--cwd',
      workDir
    ]
    if (prompt.length < 100_000) args.unshift('-p', prompt)
    else args.unshift('--prompt-file', instructions)
    return { command: 'grok', args, stdin: null, streamJson: false }
  }
  if (id === 'claude') {
    return {
      command: 'claude',
      args: [
        '-p',
        '--bare',
        '--tools',
        '',
        '--no-session-persistence',
        '--model',
        'claude-sonnet-5-5',
        '--effort',
        'low',
        '--output-format',
        'text',
        prompt
      ],
      stdin: null,
      streamJson: false
    }
  }
  if (id === 'sol') {
    return {
      command: 'codex',
      args: [
        'exec',
        '--skip-git-repo-check',
        '--ephemeral',
        '--color',
        'never',
        '-m',
        'gpt-5.6-sol',
        '-c',
        'model_reasoning_effort="low"',
        '-s',
        'read-only',
        '-C',
        workDir,
        '-'
      ],
      stdin: prompt,
      streamJson: false
    }
  }
  return {
    command: 'agy',
    args: [
      '--model',
      'gemini-3.8-flash-low',
      '--effort',
      'low',
      '--output-format',
      'text',
      '--disable-slash-commands',
      '--print',
      prompt
    ],
    stdin: null,
    streamJson: false
  }
}

export interface CaptionFixRun {
  done: Promise<{ segments?: TranscriptSegment[]; error?: string }>
  kill: () => void
}

export function startCaptionFix(
  agentId: CaptionAgentId,
  segments: TranscriptSegment[],
  request: CaptionEditRequest,
  onLog: (line: string) => void
): CaptionFixRun {
  const workDir = mkdtempSync(join(tmpdir(), 'a1-captions-'))
  const normalized = { ...request, wordsPerLine: clampWordsPerLine(request.wordsPerLine) }
  const beforeText = captionFileText(segments)
  const prompt = captionEditPrompt(normalized, beforeText)
  writeFileSync(join(workDir, 'instructions.md'), prompt, 'utf-8')
  const launch = agentLaunch(agentId, prompt, workDir)
  const agent = CAPTION_AGENTS.find((item) => item.id === agentId)
  const label = agent?.label ?? 'The agent'
  const startedAt = Date.now()
  let lastLog = startedAt
  const say = (line: string): void => {
    lastLog = Date.now()
    onLog(line)
  }
  say(`Starting ${label}.`)
  say(requestSummary(normalized))
  say('Editing the transcript.')

  const child = spawn(launch.command, launch.args, {
    cwd: workDir,
    env: process.env,
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true
  })
  child.stdin?.on('error', () => {
    // Ignore EPIPE. A closed pipe must not crash the app.
  })

  const state = emptyAgentLogState()
  const logState = { hidingFile: false }
  let killed = false
  const killTimer = { id: null as ReturnType<typeof setTimeout> | null }
  wireOutput(child, state, logState, say)

  const done = new Promise<{ segments?: TranscriptSegment[]; error?: string }>((resolve) => {
    let settled = false
    const heartbeat = setInterval(() => {
      if (settled || Date.now() - lastLog < 15_000) return
      const seconds = Math.round((Date.now() - startedAt) / 1000)
      say(`Still working. ${seconds} seconds so far.`)
    }, 5_000)
    const finish = (result: { segments?: TranscriptSegment[]; error?: string }): void => {
      if (settled) return
      settled = true
      clearInterval(heartbeat)
      if (killTimer.id) clearTimeout(killTimer.id)
      cleanup(workDir)
      resolve(result)
    }

    child.on('error', (err) => {
      const missing = (err as NodeJS.ErrnoException).code === 'ENOENT'
      finish({
        error: missing
          ? `${label} is no longer available on this computer.`
          : `${label} could not start.`
      })
    })
    child.on('close', (code) => {
      if (killed) {
        finish({ error: 'Stopped.' })
        return
      }
      const output = agentOutputText(state)
      if (isNoneAnswer(output)) {
        finish({ segments })
        return
      }
      const printed = extractPrintedCaption(output)
      if (!printed) {
        if (code && code !== 0 && !output.trim()) {
          finish({ error: `${label} stopped before it returned the lines.` })
          return
        }
        finish({ error: 'The agent did not return the edited lines. The lines were left as they are.' })
        return
      }
      if (normalizeCaptionFile(printed) === normalizeCaptionFile(beforeText)) {
        finish({ error: 'The agent did not change the file. The lines were left as they are.' })
        return
      }
      say('Updated the transcript.')
      const accepted = applyEditedCaptionText(segments, printed)
      if ('error' in accepted) {
        finish({ error: accepted.error })
        return
      }
      finish({ segments: accepted.segments })
    })
  })

  try {
    if (launch.stdin) child.stdin?.write(launch.stdin)
    child.stdin?.end()
  } catch {
    child.kill('SIGTERM')
  }

  return {
    done,
    kill: () => {
      killed = true
      say('Stopping.')
      stopChild(child, killTimer)
    }
  }
}

function normalizeCaptionFile(text: string): string {
  return text.replace(/\r\n/g, '\n').trim()
}

function requestSummary(request: CaptionEditRequest): string {
  const jobs: string[] = []
  if (request.fixTypos) jobs.push('fix typos')
  if (request.breakLines) jobs.push(`break lines longer than ${request.wordsPerLine} words`)
  if (request.note.trim()) jobs.push('follow the note you wrote')
  if (jobs.length === 0) return 'Asked it to leave the words alone.'
  if (jobs.length === 1) return `Asked it to ${jobs[0]}.`
  return `Asked it to ${jobs.slice(0, -1).join(', ')} and ${jobs[jobs.length - 1]}.`
}

function wireOutput(
  child: ChildProcess,
  state: AgentLogState,
  logState: { hidingFile: boolean },
  onLog: (line: string) => void
): void {
  const listen = (stream: NodeJS.ReadableStream | null, asAgent: boolean): void => {
    if (!stream) return
    let buffer = ''
    stream.setEncoding('utf8')
    const take = (line: string): void => {
      if (asAgent) {
        state.pieces.push(line)
        const shown = captionLogLine(line, logState)
        if (shown) onLog(shown)
        return
      }
      const shown = line.replace(/\u001b\[[0-9;]*m/g, '').trim()
      if (shown) onLog(shown)
    }
    stream.on('data', (chunk: string) => {
      buffer += chunk
      const lines = buffer.split(/\r?\n/)
      buffer = lines.pop() ?? ''
      for (const line of lines) take(line)
    })
    stream.on('end', () => {
      if (buffer) take(buffer)
      buffer = ''
    })
  }
  listen(child.stdout, true)
  listen(child.stderr, false)
}

function stopChild(child: ChildProcess, timer: { id: ReturnType<typeof setTimeout> | null }): void {
  child.kill('SIGTERM')
  timer.id = setTimeout(() => child.kill('SIGKILL'), 1000)
}

function cleanup(workDir: string): void {
  try {
    rmSync(workDir, { recursive: true, force: true })
  } catch {
    // The temp folder is only the prompt. Leaving it behind is harmless.
  }
}
