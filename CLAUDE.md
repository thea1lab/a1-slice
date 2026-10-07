# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```bash
cargo run -p a1slice          # Open the window
cargo build -p a1slice        # Compile the app
cargo test --offline          # Unit tests. One ffmpeg integration test is ignored.
bash scripts/build-whisper.sh # Build whisper-cli into resources/bin/
```

Requires Rust 1.85 or newer. `ffmpeg` and `ffprobe` must be on `PATH`. `ffplay` is optional and is used for playback audio. Offline builds use `cargo build -p a1slice --offline` when the Cargo cache is already filled.

## Architecture

Native desktop app. Two crates:

- **`crates/a1slice`** — eframe/egui window. `main.rs` opens the window. `app.rs` draws the screens. `backend.rs` runs ffmpeg, whisper-cli, and the installed agents.
- **`crates/a1slice-core`** — sidecar JSON, crop math, clip bounds, preview timing, wizard state, and settings. The window calls this crate. The library does not spawn ffmpeg.

### State

`app.rs` loads settings, opens a video, and draws the current screen from the wizard state in `a1slice-core`. Screens: `home`, `transcribe`, `transcribe-done`, `find`, `review`, `export`, `reframe`, `captions`, `fix-words`. Home opens one of four tools (`transcribe`, `find`, `reframe`, `captions`) and the video is chosen on the next screen. Pipeline stages: `idle | extracting | downloading | transcribing | analyzing | cutting | done | error`.

### Processing

1. **FFmpeg** (`backend.rs`) — extracts audio to WAV (16 kHz mono, with `afftdn` denoising), cuts clips, reframes, and burns captions. `ffprobe` reads duration and picture size.
2. **Whisper.cpp** — `scripts/build-whisper.sh` builds `whisper-cli`. The large-v3 model must already be at `~/.a1slice/models/ggml-large-v3.bin`. The app does not download it.
3. **Clip finding** — runs an installed agent (`grok`, `claude`, `codex`, or `agy`). The parser accepts the last JSON clip list the agent prints. The screens do not collect an API key.
4. **Caption edits** — Fix the words, using the same installed agents.

### Files

Language and transcription tuning live in `~/.a1slice/settings.json`. The Whisper model is at `~/.a1slice/models/ggml-large-v3.bin`. Beside the source video: `{stem}-transcript.txt`, `{stem}.a1slice.json`, `{stem}.a1slice-clips.json`, `{stem}.a1slice-analysis.txt`, `{stem}.a1slice-framing.json`, `{stem}.a1slice-captions.json`.

Find-best-parts export re-encodes each kept clip (libx264) and writes a sidecar `.srt` (subtitle mode `srt`, original picture). If that encode fails, it falls back to stream copy. Captions export burns the words into a new mp4. Reframe export writes one cropped file.

## Styling

The visual system is `DESIGN.md`, drawn in `crates/a1slice/src/app.rs`: charcoal window `#161616`, cream text, orange `#fa520f` for the next action. There is no light theme. Fonts load from the Noto Sans and Noto Serif files on the machine.

## Platform-specific notes

- Whisper binaries in `resources/bin/` are platform-specific (mac-arm64, mac-x64, linux-x64, linux-arm64, win-x64). `scripts/build-whisper.sh` builds CPU and Metal on macOS, and CPU only on Linux and Windows. They are gitignored.
- `backend.rs` looks for the binary in `resources/bin` and in `CARGO_MANIFEST_DIR/../../resources/bin`, and tells the user to run `scripts/build-whisper.sh` if it is missing.
