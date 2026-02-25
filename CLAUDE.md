# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
npm run dev          # Start electron-vite dev server with hot reload
npm run build        # Compile to out/
npm run dist         # Build + package installer for current platform
npm run dist:mac     # macOS DMG
npm run dist:win     # Windows NSIS
npm run dist:linux   # Linux AppImage
npm run test         # Run vitest once
npm run test:watch   # Vitest in watch mode
npm run build:whisper # Build whisper.cpp binaries (bash script)
```

Requires Node.js >= 24.13.1. Uses npm (not yarn/pnpm).

## Architecture

**Electron app** with strict process separation:

- **Main process** (`src/main/`) — Node.js runtime handling video processing, whisper transcription, LLM calls, file I/O. All heavy work runs here.
- **Renderer process** (`src/renderer/`) — React 19 UI in a sandboxed BrowserWindow. Cannot access Node.js APIs directly.
- **Preload bridge** (`src/main/preload.ts`) — `contextBridge.exposeInMainWorld('api', {...})` exposes typed IPC methods to the renderer as `window.api.*`.
- **Shared types** (`src/shared/types.ts`) — TypeScript interfaces used by both processes (pipeline stages, segments, settings, IPC result types).

### IPC Pattern

Main process registers handlers with `secureHandle()` (validates sender). Renderer calls them via `window.api.methodName()`. Progress updates flow back via `pipeline-progress` IPC event.

When adding a new IPC method, update all three files: `preload.ts` (implementation), `env.d.ts` (Window.api type), and `main.ts` (handler).

### State Management

`App.tsx` uses a single `useReducer` wizard state machine. Steps: `select → transcribe → review-transcript → review-slices → export`. Pipeline stages: `idle | extracting | downloading | transcribing | analyzing | cutting | done | error`.

### Processing Pipeline

1. **FFmpeg** (`ffmpeg.ts`) — extracts audio to WAV (16kHz mono, with `afftdn` denoising), splits into 3-min chunks, cuts final clips
2. **Whisper.cpp** (`whisper.ts`) — local transcription with GPU→CPU fallback, per-chunk timeout + retry, model auto-download from HuggingFace
3. **LLM Analyzer** (`analyzer.ts`) — sends transcript to Claude or OpenAI to identify clip boundaries

### Settings

Persisted to `~/.a1slice/settings.json`. Whisper model cached at `~/.a1slice/models/`. Transcripts/analyses cached alongside source video files as `.a1slice.json` / `.a1slice-clips.json`.

## Styling

Tailwind CSS v4 with `@tailwindcss/vite` plugin. Custom theme in `App.css` using `@theme` directive (dark palette, orange accent `rgb(240, 154, 62)`).

## TypeScript

Strict mode enabled. Three tsconfig project references: `tsconfig.main.json`, `tsconfig.preload.json`, `tsconfig.renderer.json`. All target ES2022 with bundler module resolution.

## Platform-Specific Notes

- Whisper binaries in `resources/bin/` are platform-specific (mac-arm64, mac-x64, linux-x64, linux-arm64, win-x64) with GPU and CPU variants
- `ffmpeg-static` and whisper binaries are unpacked from asar (`asarUnpack` in electron-builder.yml)
- Binary paths use `.replace('app.asar', 'app.asar.unpacked')` for production access
- Custom `a1slice://` protocol serves video files for in-app preview with range request support
