import { spawn } from 'child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { parseLLMResponse } from './analyzer'
import { agentLaunch, CAPTION_AGENTS, type CaptionAgentId } from './captionAgent'
import { refineClipBounds } from '../shared/clipBounds'
import { clipFindPrompt } from '../shared/clipFind'
import type { ClipSegment, TranscriptSegment } from '../shared/types'

export function clipsFromAgentText(text: string, segments: TranscriptSegment[]): ClipSegment[] {
  return parseLLMResponse(text, segments)
    .map((clip) => {
      // When two lines meet, the shared instant belongs to the earlier line.
      // Step inside the chosen line so the snap stays on that line.
      const startMs = clip.startMs + 1
      return refineClipBounds({ ...clip, startMs }, segments)
    })
    .filter((clip) => clip.endMs > clip.startMs)
}

export interface ClipFindRun {
  done: Promise<{ clips?: ClipSegment[]; raw?: string; error?: string }>
  kill: () => void
}

export function startClipFind(
  agentId: CaptionAgentId,
  segments: TranscriptSegment[],
  userHint: string | undefined,
  onLog: (line: string) => void,
  spawnProcess: typeof spawn = spawn
): ClipFindRun {
  const workDir = mkdtempSync(join(tmpdir(), 'a1-find-clips-'))
  const prompt = clipFindPrompt(segments, userHint)
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
  say('Reading the transcript.')

  const child = spawnProcess(launch.command, launch.args, {
    cwd: workDir,
    env: process.env,
    stdio: ['pipe', 'pipe', 'pipe'],
    windowsHide: true
  })
  child.stdin?.on('error', () => {
    // Ignore EPIPE. A closed pipe must not crash the app.
  })

  let stdout = ''
  let stderr = ''
  collect(child.stdout, (chunk) => {
    stdout += chunk
  })
  collect(child.stderr, (chunk) => {
    stderr += chunk
  })

  let killed = false
  const killTimer = { id: null as ReturnType<typeof setTimeout> | null }
  const done = new Promise<{ clips?: ClipSegment[]; raw?: string; error?: string }>((resolve) => {
    let settled = false
    const heartbeat = setInterval(() => {
      if (settled || Date.now() - lastLog < 15_000) return
      const seconds = Math.round((Date.now() - startedAt) / 1000)
      say(`Still working. ${seconds} seconds so far.`)
    }, 5_000)
    const finish = (result: { clips?: ClipSegment[]; raw?: string; error?: string }): void => {
      if (settled) return
      settled = true
      clearInterval(heartbeat)
      if (killTimer.id) clearTimeout(killTimer.id)
      try {
        rmSync(workDir, { recursive: true, force: true })
      } catch {
        // The temp folder is only the prompt.
      }
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
    child.on('close', () => {
      if (killed) {
        finish({ error: 'Stopped.' })
        return
      }
      const output = stdout.trim() ? stdout : stderr
      if (!output.trim()) {
        finish({ error: `${label} stopped before it returned the clips.` })
        return
      }
      try {
        const clips = clipsFromAgentText(output, segments)
        if (clips.length > 0) say(clips.length === 1 ? 'Found 1 clip.' : `Found ${clips.length} clips.`)
        finish({ clips, raw: output })
      } catch {
        finish({ error: 'The agent did not return any clips.' })
      }
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
      killTimer.id = setTimeout(() => child.kill('SIGKILL'), 1000)
      child.kill('SIGTERM')
    }
  }
}

function collect(
  stream: NodeJS.ReadableStream | null,
  take: (chunk: string) => void
): void {
  if (!stream) return
  stream.setEncoding?.('utf8')
  stream.on('data', (chunk: string) => take(chunk))
}
