# A1 Slice — build notes

The product description and how to run the app are in [README.md](README.md). This file is the build and layout detail.

## Requirements

| | Minimum | Comfortable |
| --- | --- | --- |
| RAM | 8 GB | 16 GB |
| CPU | Modern quad-core | 8 cores |
| Disk | 4 GB free for the Whisper model, plus room for the video and exports | |
| OS | Windows 10, macOS 12, Ubuntu 22.04 | Current release |

Rust 1.85 or newer. `ffmpeg` and `ffprobe` must be on `PATH`.

Transcription uses whisper.cpp **large-v3**. `scripts/build-whisper.sh` makes a CPU binary on Linux and Windows, and CPU plus Metal binaries on macOS.

## Whisper

`resources/bin/` is gitignored except for `.gitkeep`. Build the program with:

```bash
bash scripts/build-whisper.sh
```

That clones whisper.cpp v1.8.3, builds `whisper-cli`, and copies it into place. You need git, CMake, and a C++ compiler.

| Platform | File |
| --- | --- |
| Windows x64 | `whisper-cli-win-x64.exe` |
| macOS Apple Silicon | `whisper-cli-mac-arm64` and `whisper-cli-mac-arm64-gpu` |
| macOS Intel | `whisper-cli-mac-x64` and `whisper-cli-mac-x64-gpu` |
| Linux x64 | `whisper-cli-linux-x64` |
| Linux arm64 | `whisper-cli-linux-arm64` |

The app looks in `resources/bin/` from the working directory, then next to the crate at `crates/a1slice/../../resources/bin`. It prefers the GPU binary on macOS and falls back to the CPU binary.

The model is a separate download, about 3 GB. Save it as `~/.a1slice/models/ggml-large-v3.bin`:

```bash
mkdir -p ~/.a1slice/models
curl -L -o ~/.a1slice/models/ggml-large-v3.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin
```

The file is `ggml-large-v3.bin` from [ggerganov/whisper.cpp on Hugging Face](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin). Its size is 3,094,623,232 bytes.

## Layout

```
crates/
  a1slice/            eframe window
    src/main.rs       Window entry
    src/app/          One file per screen, plus the player and widgets
    src/backend/      ffmpeg, whisper-cli, and installed agents
  a1slice-core/       Sidecar files, crop, clips, settings, wizard state
resources/
  bin/                whisper.cpp binaries (built, not committed)
  icon.*              App icons
scripts/
  build-whisper.sh    Builds whisper-cli for this machine
```

Screens are `home`, `transcribe`, `transcribe-done`, `find`, `review`, `export`, `reframe`, `captions`, and `fix-words`.

## Checks

```bash
cargo test --offline
cargo build -p a1slice --offline
```

Run the window with `cargo run -p a1slice`.

## License

MIT. See [LICENSE](LICENSE).
