import { join } from 'path'
import { homedir, platform, arch } from 'os'
import { existsSync, mkdirSync, createWriteStream, readFileSync, chmodSync, unlinkSync, renameSync } from 'fs'
import { spawn } from 'child_process'
import https from 'https'
import http from 'http'
import type { TranscriptSegment } from '../shared/types'

const MODEL_FILENAME = 'ggml-large-v3.bin'
const MODEL_URL =
  'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin'

// Built from whisper.cpp v1.8.3 — rebuild via:
//   gh workflow run build-whisper.yml -f whisper_ref=<tag> -f release_tag=whisper-<version>
const WHISPER_BINARY_VERSION = 'v1.0.0'
const WHISPER_BINARY_BASE_URL =
  'https://github.com/a1lab/a1-slice/releases/download/whisper-' + WHISPER_BINARY_VERSION

function getModelsDir(): string {
  const dir = join(homedir(), '.a1slice', 'models')
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function getBinDir(): string {
  const dir = join(homedir(), '.a1slice', 'bin', `whisper-${WHISPER_BINARY_VERSION}`)
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true })
  return dir
}

function getWhisperBinaryName(): string {
  const os = platform()
  const cpu = arch()

  if (os === 'win32') {
    return 'whisper-cli-win-x64.exe'
  } else if (os === 'darwin') {
    return cpu === 'arm64' ? 'whisper-cli-mac-arm64' : 'whisper-cli-mac-x64'
  } else {
    return cpu === 'arm64' ? 'whisper-cli-linux-arm64' : 'whisper-cli-linux-x64'
  }
}

export function getWhisperBinaryPath(): string {
  const name = getWhisperBinaryName()

  // 1. User cache (~/.a1slice/bin/<version>/)
  const cachedPath = join(getBinDir(), name)
  if (existsSync(cachedPath)) return cachedPath

  // 2. Production (process.resourcesPath/bin/)
  const prodPath = join(process.resourcesPath ?? '', 'bin', name)
  if (existsSync(prodPath)) return prodPath

  // 3. Dev (resources/bin/)
  const devPath = join(__dirname, '../../resources/bin', name)
  return devPath.replace('app.asar', 'app.asar.unpacked')
}

export function downloadWhisperBinary(
  onProgress?: (percent: number) => void
): Promise<string> {
  const name = getWhisperBinaryName()
  const binDir = getBinDir()
  const binaryPath = join(binDir, name)

  if (existsSync(binaryPath)) return Promise.resolve(binaryPath)

  const url = `${WHISPER_BINARY_BASE_URL}/${name}`
  const partialPath = binaryPath + '.partial'

  return new Promise((resolve, reject) => {
    function doRequest(reqUrl: string): void {
      const proto = reqUrl.startsWith('https') ? https : http
      proto
        .get(reqUrl, (res) => {
          if (
            (res.statusCode === 301 || res.statusCode === 302) &&
            res.headers.location
          ) {
            doRequest(res.headers.location)
            return
          }

          if (res.statusCode !== 200) {
            reject(new Error(`Binary download failed: HTTP ${res.statusCode}`))
            return
          }

          const totalBytes = parseInt(res.headers['content-length'] ?? '0', 10)
          let downloaded = 0
          const file = createWriteStream(partialPath)

          res.on('data', (chunk: Buffer) => {
            downloaded += chunk.length
            if (onProgress && totalBytes > 0) {
              onProgress(Math.round((downloaded / totalBytes) * 100))
            }
          })

          res.pipe(file)
          file.on('finish', () => {
            file.close()
            try {
              // Rename partial to final
              renameSync(partialPath, binaryPath)
              // Make executable on Unix
              if (platform() !== 'win32') {
                chmodSync(binaryPath, 0o755)
              }
              resolve(binaryPath)
            } catch (err) {
              reject(err)
            }
          })
          file.on('error', (err) => {
            // Clean up partial file on error
            try { unlinkSync(partialPath) } catch {}
            reject(err)
          })
        })
        .on('error', (err) => {
          try { unlinkSync(partialPath) } catch {}
          reject(err)
        })
    }

    doRequest(url)
  })
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
