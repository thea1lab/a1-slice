import { join } from 'path'
import { homedir, platform, arch } from 'os'
import { existsSync, mkdirSync, createWriteStream, readFileSync, renameSync, unlinkSync, statSync } from 'fs'
import { spawn } from 'child_process'
import https from 'https'
import http from 'http'
import type { TranscriptSegment } from '../shared/types'

const MODEL_FILENAME = 'ggml-large-v3.bin'
const MODEL_URL =
  'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin'
const EXPECTED_MODEL_BYTES = 3_094_623_232 // ggml-large-v3.bin known size
const MODEL_SIZE_TOLERANCE = 0.99 // accept if >= 99% of expected

function getModelsDir(): string {
  const dir = join(homedir(), '.a1slice', 'models')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function getWhisperBinaryNames(): [string, string] {
  const os = platform()
  const cpu = arch()
  let base: string

  if (os === 'win32') base = 'whisper-cli-win-x64'
  else if (os === 'darwin') base = cpu === 'arm64' ? 'whisper-cli-mac-arm64' : 'whisper-cli-mac-x64'
  else base = cpu === 'arm64' ? 'whisper-cli-linux-arm64' : 'whisper-cli-linux-x64'

  const ext = os === 'win32' ? '.exe' : ''
  return [`${base}-gpu${ext}`, `${base}${ext}`]
}

function findBinaryPath(name: string): string | null {
  const prodPath = join(process.resourcesPath ?? '', 'bin', name)
  if (existsSync(prodPath)) return prodPath
  const devPath = join(__dirname, '../../resources/bin', name).replace('app.asar', 'app.asar.unpacked')
  if (existsSync(devPath)) return devPath
  return null
}

export function getWhisperBinaryPath(): string {
  const [gpuName, cpuName] = getWhisperBinaryNames()
  return findBinaryPath(gpuName) ?? findBinaryPath(cpuName) ?? join(__dirname, '../../resources/bin', cpuName).replace('app.asar', 'app.asar.unpacked')
}

const RETRYABLE_CODES = new Set([
  'ECONNRESET', 'ETIMEDOUT', 'ENOTFOUND', 'EPIPE',
  'EAI_AGAIN', 'ECONNREFUSED', 'EHOSTUNREACH'
])
const MAX_RETRIES = 3
const BASE_DELAY_MS = 2000

function isRetryableError(err: unknown): boolean {
  if (!(err instanceof Error)) return false
  const code = (err as NodeJS.ErrnoException).code
  if (code && RETRYABLE_CODES.has(code)) return true
  if (err.message === 'socket hang up' || err.message === 'Download timed out') return true
  return false
}

function delay(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

function attemptDownload(
  modelPath: string,
  partialPath: string,
  onProgress?: (percent: number) => void
): Promise<string> {
  return new Promise((resolve, reject) => {
    let settled = false
    function fail(err: Error): void {
      if (settled) return
      settled = true
      reject(err)
    }

    let existingBytes = 0
    try {
      existingBytes = statSync(partialPath).size
    } catch { /* no partial file yet */ }

    function doRequest(url: string, redirects = 0): void {
      if (redirects > 5) {
        fail(new Error('Too many redirects'))
        return
      }

      const headers: Record<string, string> = {}
      if (existingBytes > 0) {
        headers['Range'] = `bytes=${existingBytes}-`
      }

      const proto = url.startsWith('https') ? https : http
      const req = proto.get(url, { timeout: 30_000, headers }, (res) => {
        // Handle redirects (301, 302, 307, 308)
        const sc = res.statusCode ?? 0
        if ([301, 302, 307, 308].includes(sc) && res.headers.location) {
          res.resume() // consume body so socket can be freed
          doRequest(res.headers.location, redirects + 1)
          return
        }

        // 416 Range Not Satisfiable — range exceeds file size
        if (sc === 416) {
          res.resume()
          try {
            const partialSize = statSync(partialPath).size
            // Only treat as complete if size matches expected model size
            if (partialSize >= EXPECTED_MODEL_BYTES * MODEL_SIZE_TOLERANCE) {
              renameSync(partialPath, modelPath)
              if (!settled) { settled = true; resolve(modelPath) }
              return
            }
          } catch { /* ignore */ }
          // Partial file is wrong size — delete and retry from scratch
          try { unlinkSync(partialPath) } catch { /* ignore */ }
          fail(new Error('Range not satisfiable, restarting download'))
          return
        }

        let downloaded: number
        let totalBytes: number

        if (sc === 206) {
          // Partial content — append
          downloaded = existingBytes
          const rangeHeader = res.headers['content-range'] // e.g. "bytes 1234-5678/9999"
          if (rangeHeader) {
            const match = rangeHeader.match(/\/(\d+)/)
            totalBytes = match ? parseInt(match[1], 10) : 0
          } else {
            const cl = parseInt(res.headers['content-length'] ?? '0', 10)
            totalBytes = existingBytes + cl
          }
        } else if (sc === 200) {
          // Server ignored Range — restart from scratch
          if (existingBytes > 0) {
            try { unlinkSync(partialPath) } catch { /* ignore */ }
            existingBytes = 0
          }
          downloaded = 0
          totalBytes = parseInt(res.headers['content-length'] ?? '0', 10)
        } else {
          fail(new Error(`Download failed: HTTP ${sc}`))
          return
        }

        const fileFlags = sc === 206 ? 'a' : 'w'
        const file = createWriteStream(partialPath, { flags: fileFlags })

        res.on('data', (chunk: Buffer) => {
          downloaded += chunk.length
          if (onProgress && totalBytes > 0) {
            onProgress(Math.round((downloaded / totalBytes) * 100))
          }
        })

        res.on('error', (err) => {
          file.destroy()
          fail(err)
        })

        res.pipe(file)

        file.on('finish', () => {
          file.close(() => {
            if (settled) return
            settled = true
            try {
              renameSync(partialPath, modelPath)
              resolve(modelPath)
            } catch (err) {
              fail(err as Error)
            }
          })
        })
        file.on('error', (err) => {
          res.destroy()
          fail(err)
        })
      })

      req.on('error', (err) => fail(err))
      req.on('timeout', () => {
        req.destroy()
        fail(new Error('Download timed out'))
      })
    }

    doRequest(MODEL_URL)
  })
}

export async function downloadModel(
  onProgress?: (percent: number) => void
): Promise<string> {
  const modelPath = join(getModelsDir(), MODEL_FILENAME)

  // Validate existing model — delete if truncated/corrupt
  if (existsSync(modelPath)) {
    try {
      const size = statSync(modelPath).size
      if (size >= EXPECTED_MODEL_BYTES * MODEL_SIZE_TOLERANCE) {
        return modelPath
      }
      // Too small — corrupted download, remove and re-download
      unlinkSync(modelPath)
    } catch {
      try { unlinkSync(modelPath) } catch { /* ignore */ }
    }
  }

  const partialPath = modelPath + '.partial'

  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    if (attempt > 0) {
      await delay(BASE_DELAY_MS * Math.pow(2, attempt - 1))
    }
    try {
      return await attemptDownload(modelPath, partialPath, onProgress)
    } catch (err) {
      if (!isRetryableError(err) || attempt === MAX_RETRIES) {
        try { unlinkSync(partialPath) } catch { /* ignore */ }
        if (attempt === MAX_RETRIES) {
          throw new Error(
            `Download failed after ${MAX_RETRIES + 1} attempts: ${(err as Error).message}`
          )
        }
        throw err
      }
      // retryable — keep .partial for resume, loop continues
    }
  }

  // unreachable, but satisfies TypeScript
  throw new Error('Download failed')
}

export function parseTimestamp(ts: string): number {
  // Handles "HH:MM:SS.mmm" or "HH:MM:SS,mmm"
  const parts = ts.replace(',', '.').split(':')
  const hours = parseInt(parts[0], 10)
  const minutes = parseInt(parts[1], 10)
  const secParts = parts[2].split('.')
  const seconds = parseInt(secParts[0], 10)
  const millis = parseInt((secParts[1] ?? '0').padEnd(3, '0').slice(0, 3), 10)
  return hours * 3600000 + minutes * 60000 + seconds * 1000 + millis
}

export function parseWhisperJson(
  json: string
): TranscriptSegment[] {
  const data = JSON.parse(json)
  const results: TranscriptSegment[] = data.transcription.map(
    (entry: { timestamps: { from: string; to: string }; text: string }) => ({
      startMs: parseTimestamp(entry.timestamps.from),
      endMs: parseTimestamp(entry.timestamps.to),
      text: entry.text
    })
  )
  return results
}

function runWhisper(
  binaryPath: string,
  wavPath: string,
  onProgress?: (percent: number) => void
): Promise<TranscriptSegment[]> {
  return new Promise((resolve, reject) => {
    const modelPath = join(getModelsDir(), MODEL_FILENAME)
    const outputBase = wavPath.replace(/\.wav$/, '')
    const args = [
      '-m',
      modelPath,
      '-f',
      wavPath,
      '-oj', // JSON output
      '-of',
      outputBase,
      '-pp' // print progress to stderr
    ]

    const proc = spawn(binaryPath, args)
    let stderr = ''

    proc.stderr.on('data', (chunk: Buffer) => {
      stderr += chunk.toString()
      // whisper.cpp prints progress like "whisper_full: progress = 42%"
      const match = stderr.match(/progress\s*=\s*(\d+)%/g)
      if (match && onProgress) {
        const last = match[match.length - 1]
        const pct = parseInt(last.match(/(\d+)/)![1], 10)
        onProgress(pct)
      }
    })

    proc.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(`whisper-cli exited with code ${code}: ${stderr}`))
        return
      }
      try {
        const jsonPath = outputBase + '.json'
        const raw = readFileSync(jsonPath, 'utf-8')
        resolve(parseWhisperJson(raw))
      } catch (err) {
        reject(err)
      }
    })

    proc.on('error', (err) => reject(err))
  })
}

export function transcribe(
  wavPath: string,
  onProgress?: (percent: number) => void
): Promise<TranscriptSegment[]> {
  const modelPath = join(getModelsDir(), MODEL_FILENAME)
  if (!existsSync(modelPath)) {
    return Promise.reject(new Error('Whisper model not found. Download it first.'))
  }

  const [gpuName, cpuName] = getWhisperBinaryNames()
  const gpuPath = findBinaryPath(gpuName)
  const cpuPath = findBinaryPath(cpuName)

  const tryBinary = (binaryPath: string) => runWhisper(binaryPath, wavPath, onProgress)

  if (gpuPath) {
    return tryBinary(gpuPath).catch(() => {
      if (cpuPath) return tryBinary(cpuPath)
      throw new Error('No whisper binary available')
    })
  }
  if (cpuPath) return tryBinary(cpuPath)
  return Promise.reject(new Error('No whisper binary found'))
}
