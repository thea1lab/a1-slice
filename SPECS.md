# A1 Slice — build notes

The product description and how to run the app are in [README.md](README.md). This file is the build and layout detail.

## Requirements

| | Minimum | Comfortable |
| --- | --- | --- |
| RAM | 8 GB | 16 GB |
| CPU | Modern quad-core | 8 cores |
| Disk | 4 GB free for the Whisper model, plus room for the video and exports | |
| OS | Windows 10, macOS 12, Ubuntu 22.04 | Current release |

Transcription uses whisper.cpp **large-v3**. A GPU build is optional. `pnpm build:whisper` makes a CPU binary on Linux and Windows, and CPU plus Metal binaries on macOS. The Docker image in `Dockerfile` can also build a Linux CUDA binary.

Node.js is pinned to 24.13.1 in `.node-version`. The package manager is pnpm 12 (`packageManager` in `package.json`).

## Whisper binaries

`resources/bin/` is gitignored except for `.gitkeep`. Build them with:

```bash
pnpm build:whisper
```

That clones whisper.cpp v1.8.3, builds `whisper-cli`, and copies it into place. You need git, CMake, and a C++ compiler.

| Platform | File |
| --- | --- |
| Windows x64 | `whisper-cli-win-x64.exe` |
| macOS Apple Silicon | `whisper-cli-mac-arm64` and `whisper-cli-mac-arm64-gpu` |
| macOS Intel | `whisper-cli-mac-x64` and `whisper-cli-mac-x64-gpu` |
| Linux x64 | `whisper-cli-linux-x64` |
| Linux arm64 | `whisper-cli-linux-arm64` |

The release workflow builds the two macOS binaries when the `package.json` version changes on `main`. It publishes a DMG. It does not build Windows or Linux. Those installers are produced locally (`pnpm dist:win`, `pnpm dist:linux`, or the Docker build) and uploaded with `scripts/upload-release.sh`.

## Layout

```
src/
  main/           Electron main process
    main.ts       Window and IPC
    preload.ts    Bridge exposed as window.api
    whisper.ts    Model download and transcription
    ffmpeg.ts     Audio extract, clip cut, reframe, caption burn
    clipAgent.ts  Find best parts, via an installed agent
    captionAgent.ts  Fix the words, via the same agents
    analyzer.ts   Parses the agent's clip JSON
    projectStore.ts  Reads and writes the files beside the video
  renderer/       React UI
    screens/      One screen per tool
    App.tsx       Settings, open video, current screen
    state.ts      Reducer
  shared/         Types and pure helpers used by both processes
resources/bin/    whisper.cpp binaries (built, not committed)
```

Screens are `home`, `transcribe`, `transcribe-done`, `find`, `review`, `export`, `reframe`, `captions`, and `fix-words`.

## Checks

```bash
pnpm test
pnpm build
```

## License

MIT. See [LICENSE](LICENSE).
