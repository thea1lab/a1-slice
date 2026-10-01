# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
pnpm dev             # Start electron-vite dev server with hot reload
pnpm build           # Compile to out/
pnpm dist            # Build + package installer for current platform
pnpm dist:mac        # macOS DMG
pnpm dist:win        # Windows NSIS
pnpm dist:linux      # Linux AppImage
pnpm test            # Run vitest once
pnpm test:watch      # Vitest in watch mode
pnpm build:whisper   # Build whisper.cpp binaries (bash script)
```

Requires Node.js >= 24.13.1. Uses pnpm (not npm/yarn).

## Architecture

**Electron app** with strict process separation:

- **Main process** (`src/main/`) — Node.js runtime handling video processing, whisper transcription, local agent calls, and file I/O. All heavy work runs here.
- **Renderer process** (`src/renderer/`) — React 19 UI in a sandboxed BrowserWindow. Cannot access Node.js APIs directly.
- **Preload bridge** (`src/main/preload.ts`) — `contextBridge.exposeInMainWorld('api', {...})` exposes typed IPC methods to the renderer as `window.api.*`.
- **Shared types** (`src/shared/types.ts`) — TypeScript interfaces used by both processes (pipeline stages, segments, settings, IPC result types).

### IPC Pattern

Main process registers handlers with `secureHandle()` (validates sender). Renderer calls them via `window.api.methodName()`. Progress updates flow back via `pipeline-progress` IPC event.

When adding a new IPC method, update all three files: `preload.ts` (implementation), `env.d.ts` (Window.api type), and `main.ts` (handler).

### State

`App.tsx` loads settings, opens a video, and renders the current screen from `state.ts` (`wizardReducer`). Screens: `home`, `transcribe`, `transcribe-done`, `find`, `review`, `export`, `reframe`, `captions`, `fix-words`. Home opens one of four tools (`transcribe`, `find`, `reframe`, `captions`) and the video is chosen on the next screen. Pipeline stages: `idle | extracting | downloading | transcribing | analyzing | cutting | done | error`.

### Processing

1. **FFmpeg** (`ffmpeg.ts`) — extracts audio to WAV (16kHz mono, with `afftdn` denoising), splits it into chunks, cuts clips, reframes, and burns captions
2. **Whisper.cpp** (`whisper.ts`) — local transcription with GPU→CPU fallback, per-chunk timeout + retry, model auto-download from HuggingFace (`ggml-large-v3.bin`)
3. **Clip finding** (`clipAgent.ts`) — runs an installed agent (`grok`, `claude`, `codex`, or `agy`). `analyzer.ts` parses the JSON it prints. The screens do not collect an API key.
4. **Caption edits** (`captionAgent.ts`) — Fix the words, using the same installed agents

### Files

Language and transcription tuning live in `~/.a1slice/settings.json`. The Whisper model is cached at `~/.a1slice/models/`. Beside the source video: `{stem}-transcript.txt`, `{stem}.a1slice.json`, `{stem}.a1slice-clips.json`, `{stem}.a1slice-analysis.txt`, `{stem}.a1slice-framing.json`, `{stem}.a1slice-captions.json`.

Find-best-parts export re-encodes each kept clip (libx264) and writes a sidecar `.srt` (subtitle mode `srt`, original picture). If that encode fails, it falls back to stream copy. Captions export burns the words into a new mp4. Reframe export writes one cropped file.

## Styling

Tailwind CSS v4 with `@tailwindcss/vite`. The visual system is `DESIGN.md`, applied through `App.css`: charcoal window `#161616`, cream text, orange `#fa520f` for the next action. There is no light theme.

## TypeScript

Strict mode enabled. Three tsconfig project references: `tsconfig.main.json`, `tsconfig.preload.json`, `tsconfig.renderer.json`. All target ES2022 with bundler module resolution.

## Platform-Specific Notes

- Whisper binaries in `resources/bin/` are platform-specific (mac-arm64, mac-x64, linux-x64, linux-arm64, win-x64). `pnpm build:whisper` builds CPU and Metal on macOS, and CPU only on Linux and Windows. They are gitignored.
- `ffmpeg-static` and whisper binaries are unpacked from asar (`asarUnpack` in electron-builder.yml)
- Binary paths use `.replace('app.asar', 'app.asar.unpacked')` for production access
- Custom `a1slice://` protocol serves video files for in-app preview with range request support
