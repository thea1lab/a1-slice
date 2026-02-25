# A1 Slice

Cut long videos into short, captioned reels using AI.

A1 Slice is a desktop app that takes long-form videos (interviews, podcasts, talks) and automatically finds the best moments, extracts them as clips, and generates subtitles — all with minimal manual effort.

## How It Works

A1 Slice walks you through a 5-step wizard:

1. **Select a video** — Pick any MP4, MOV, MKV, AVI, or WebM file and configure transcription settings.
2. **Transcribe** — Audio is extracted, denoised, and transcribed locally using Whisper.cpp. No data leaves your machine for this step.
3. **Review transcript** — Read through the full transcript, then send it to an LLM (Claude or OpenAI) to identify the best clip-worthy moments.
4. **Review clips** — Browse AI-suggested clips with titles and time ranges. Toggle clips on/off, adjust start/end times, and preview in-app.
5. **Export** — Approved clips are cut from the original video (no re-encoding) with SRT subtitle files generated for each.

## Features

- **Local transcription** — Runs Whisper.cpp on your machine with automatic GPU-to-CPU fallback. Model downloads automatically on first run.
- **AI clip detection** — Claude or OpenAI analyzes the transcript and suggests engaging clip boundaries with titles and topic grouping.
- **In-app video preview** — Preview clips before exporting.
- **SRT subtitles** — Every exported clip comes with a subtitle file.
- **Fast export** — Clips are stream-copied from the source video, so export is nearly instant.
- **Smart caching** — Transcripts and analyses are cached next to the source file. Re-opening the same video skips already-completed steps.
- **Multi-language** — Supports English, Portuguese, and Spanish (auto-detect or manual selection).
- **Cross-platform** — macOS, Windows, and Linux.

## Getting Started

Requires **Node.js >= 24.13.1**.

```bash
npm install
npm run dev
```

To build a distributable installer:

```bash
npm run dist          # Current platform
npm run dist:mac      # macOS DMG
npm run dist:win      # Windows NSIS
npm run dist:linux    # Linux AppImage
```

## Settings

On first launch, go to Settings to configure:

- **LLM provider** — Choose Claude or OpenAI and enter your API key.
- **Whisper model** — Downloads automatically (~3 GB for large-v3). Stored in `~/.a1slice/models/`.
- **Transcription tuning** — Entropy threshold, beam size, max context, and temperature controls for advanced users.

Settings are saved to `~/.a1slice/settings.json`.

## Output

Exported clips are saved to a timestamped folder alongside the source video:

```
a1slice-{videoname}-{timestamp}/
├── Clip Title 1.mp4
├── Clip Title 1.srt
├── Clip Title 2.mp4
├── Clip Title 2.srt
└── transcript.txt
```

## Built With

Electron, React 19, Tailwind CSS v4, Whisper.cpp, FFmpeg, and the Claude/OpenAI APIs.
