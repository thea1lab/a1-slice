# A1 Slice

A1 Slice is a desktop app for working on one video at a time. Home asks what you want to do. You pick a tool, then the video.

- **Transcribe** turns speech into a transcript and saves it next to the video. That step stays on this computer.
- **Find best parts** marks the moments worth keeping, lets you trim the start and end, then exports those clips with a subtitle file.
- **Reframe** crops the picture for a phone story, YouTube, a square, or classic 4:3, and remembers the frame for that video.
- **Captions** puts the words on the picture. You choose the color, size, font, and position, then export a new file.

Accepted video files are MP4, MOV, MKV, AVI, and WebM. Spoken language can be auto-detected, or set to English, Portuguese, or Spanish.

## What you need to run it

- Node.js 24.13.1 or newer (`.node-version`)
- pnpm 12
- git, CMake, and a C++ compiler, used once by `pnpm build:whisper`

On the first transcription the app downloads the Whisper large-v3 model, about 3 GB, into `~/.a1slice/models/`.

Find best parts, and Fix the words on the Captions screen, use an agent program already installed and signed in on this computer. The app looks for `grok`, `claude`, `codex`, or `agy`. Transcription, reframing, and burning captions do not use one.

## Run from source

```bash
git clone https://github.com/thea1lab/a1-slice.git
cd a1-slice
pnpm install
pnpm build:whisper
pnpm dev
```

`pnpm build:whisper` builds whisper.cpp v1.8.3 into `resources/bin/`. Those binaries are gitignored, so a fresh clone needs this step before transcription will run.

## Where the work is saved

Beside the video:

| File | Contents |
| --- | --- |
| `Name-transcript.txt` | Readable transcript |
| `Name.a1slice.json` | Working transcript |
| `Name.a1slice-clips.json` | Clips you kept |
| `Name.a1slice-framing.json` | Saved frame |
| `Name.a1slice-captions.json` | Caption source and style |

Exports are new folders beside the video:

- Clips land in `a1slice-Name-<time>/` as `01_Title.mp4`, a matching `.srt`, and `transcript.txt`. The picture stays the original frame. Subtitles are a sidecar file.
- A reframe lands in `a1slice-Name-reframe-<time>/Name.mp4`.
- Captions land in `a1slice-Name-captions-<time>/Name.mp4`, with the words burned into the picture.

Language and the “If the words come out wrong” transcription controls are stored in `~/.a1slice/settings.json`.

## Build an installer

Build the whisper binary for the target platform first. The model is not inside the installer.

```bash
pnpm dist          # this computer
pnpm dist:mac      # macOS DMG
pnpm dist:win      # Windows NSIS installer
pnpm dist:linux    # Linux AppImage
```

Output goes to `release/`. The macOS package is ad-hoc signed, so Gatekeeper will ask before it opens.

The installers attached to GitHub Release v1.0.21 (25 February 2026) are behind the source on `main`. They are a macOS DMG, a Windows setup, and a Linux AppImage of the older guided flow. Build from this tree to get the four tools above. Pushing a new `version` in `package.json` to `main` starts a release workflow that builds the macOS DMG. Windows and Linux installers are added with `scripts/upload-release.sh`.

## For contributors

Build steps, the whisper binary names, and the source layout are in [SPECS.md](SPECS.md). Notes for working in this repo are in [CLAUDE.md](CLAUDE.md). The visual system is [DESIGN.md](DESIGN.md).

## License

[MIT](LICENSE)
