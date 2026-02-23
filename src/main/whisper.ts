import { join } from 'path'
import { homedir, platform, arch } from 'os'
import { existsSync, mkdirSync, createWriteStream, readFileSync } from 'fs'
import { spawn } from 'child_process'
import https from 'https'
import http from 'http'
import type { TranscriptSegment } from '../shared/types'

const MODEL_FILENAME = 'ggml-large-v3.bin'
const MODEL_URL =
  'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin'

function getModelsDir(): string {
  const dir = join(homedir(), '.a1slice', 'models')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

export function getWhisperBinaryPath(): string {
  const os = platform()
  const cpu = arch()
  let name: string

  if (os === 'win32') {
    name = 'whisper-cli-win-x64.exe'
  } else if (os === 'darwin') {
    name = cpu === 'arm64' ? 'whisper-cli-mac-arm64' : 'whisper-cli-mac-x64'
  } else {
    name = cpu === 'arm64' ? 'whisper-cli-linux-arm64' : 'whisper-cli-linux-x64'
  }

  // In production, resources are in app.asar unpacked
  const devPath = join(__dirname, '../../resources/bin', name)
  const prodPath = join(process.resourcesPath ?? '', 'bin', name)
  return existsSync(prodPath) ? prodPath : devPath
}

export function downloadModel(
  onProgress?: (percent: number) => void
): Promise<string> {
  const modelPath = join(getModelsDir(), MODEL_FILENAME)
  if (existsSync(modelPath)) return Promise.resolve(modelPath)

  return new Promise((resolve, reject) => {
    function doRequest(url: string): void {
      const proto = url.startsWith('https') ? https : http
      proto
        .get(url, (res) => {
          // Handle redirects
          if (
            (res.statusCode === 301 || res.statusCode === 302) &&
            res.headers.location
          ) {
            doRequest(res.headers.location)
            return
          }

          if (res.statusCode !== 200) {
            reject(new Error(`Download failed: HTTP ${res.statusCode}`))
            return
          }

          const totalBytes = parseInt(res.headers['content-length'] ?? '0', 10)
          let downloaded = 0
          const file = createWriteStream(modelPath)

          res.on('data', (chunk: Buffer) => {
            downloaded += chunk.length
            if (onProgress && totalBytes > 0) {
              onProgress(Math.round((downloaded / totalBytes) * 100))
            }
          })

          res.pipe(file)
          file.on('finish', () => {
            file.close()
            resolve(modelPath)
          })
          file.on('error', (err) => reject(err))
        })
        .on('error', (err) => reject(err))
    }

    doRequest(MODEL_URL)
  })
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

export function transcribe(
  wavPath: string,
  onProgress?: (percent: number) => void
): Promise<TranscriptSegment[]> {
  return new Promise((resolve, reject) => {
    const modelPath = join(getModelsDir(), MODEL_FILENAME)
    if (!existsSync(modelPath)) {
      reject(new Error('Whisper model not found. Download it first.'))
      return
    }

    const binaryPath = getWhisperBinaryPath()
    const outputBase = wavPath.replace(/\.wav$/, '')
    const args = [
      '-m',
      modelPath,
      '-f',
      wavPath,
      '-oj', // JSON output
      '-of',
      outputBase
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
