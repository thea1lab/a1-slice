# A1 Slice

A1 Slice is a desktop app for working on one video at a time. Home asks what you want to do. You pick a tool, then the video.

- **Transcribe** turns speech into a transcript and saves it next to the video. That step stays on this computer.
- **Find best parts** marks the moments worth keeping, lets you trim the start and end, then exports those clips with a subtitle file.
- **Reframe** crops the picture for a phone story, YouTube, a square, or classic 4:3, and remembers the frame for that video.
- **Captions** puts the words on the picture. You choose the color, size, font, and position, then export a new file.

Accepted video files are MP4, MOV, MKV, AVI, and WebM. Spoken language can be auto-detected, or set to English, Portuguese, or Spanish.

## What you need to run it

- Rust 1.85 or newer
- `ffmpeg` and `ffprobe` on `PATH`. `ffplay` is used for audio during playback when it is installed.
- git, CMake, and a C++ compiler, used once by `scripts/build-whisper.sh`

Find best parts, and Fix the words on the Captions screen, use an agent program already installed and signed in on this computer. The app looks for `grok`, `claude`, `codex`, or `agy`. Transcription, reframing, and burning captions do not use one.

## Run from source

```bash
git clone https://github.com/thea1lab/a1-slice.git
cd a1-slice
bash scripts/build-whisper.sh
mkdir -p ~/.a1slice/models
curl -L -o ~/.a1slice/models/ggml-large-v3.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin
cargo run -p a1slice
```

`scripts/build-whisper.sh` builds whisper.cpp v1.8.3 into `resources/bin/`. Those binaries are gitignored, so a fresh clone needs this step before transcription will run. On macOS it builds a CPU binary and a Metal binary. On Linux and Windows it builds a CPU binary.

The Whisper large-v3 model is about 3 GB. Put it at `~/.a1slice/models/ggml-large-v3.bin`. Transcription stops with a clear error until that file is there.

## Where the work is saved

Beside the video:

| File | Contents |
| --- | --- |
| `Name-transcript.txt` | Readable transcript |
| `Name.a1slice.json` | Working transcript |
| `Name.a1slice-clips.json` | Clips you kept |
| `Name.a1slice-analysis.txt` | Notes from Find best parts |
| `Name.a1slice-framing.json` | Saved frame |
| `Name.a1slice-captions.json` | Caption source and style |

Exports are new folders beside the video:

- Clips land in `a1slice-Name-<time>/` as `01_Title.mp4`, a matching `.srt`, and `transcript.txt`. The picture stays the original frame. Subtitles are a sidecar file.
- A reframe lands in `a1slice-Name-reframe-<time>/Name.mp4`.
- Captions land in `a1slice-Name-captions-<time>/Name.mp4`, with the words burned into the picture.

Language and the “If the words come out wrong” transcription controls are stored in `~/.a1slice/settings.json`.

## For contributors

Build steps, the whisper binary names, and the source layout are in [SPECS.md](SPECS.md). Notes for working in this repo are in [CLAUDE.md](CLAUDE.md). The visual system is [DESIGN.md](DESIGN.md).

## License

[MIT](LICENSE)
