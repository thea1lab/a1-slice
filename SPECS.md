# ✂️ A1 Slice — by A1 Lab



**Cut long videos into short, captioned reels using AI — fully local, open source, no subscription.**



You bring a video. A1 Slice transcribes it with Whisper, sends the transcript to an AI model (Claude or GPT-4o), and cuts the best self-contained clips with subtitles burned in — ready to post.



---



## ⚙️ System Requirements



> ⚠️ **This app is CPU/GPU intensive. A good machine is required.**



| Component | Minimum | Recommended |

|-----------|---------|-------------|

| RAM | 8 GB | 16 GB+ |

| CPU | Modern quad-core | 8-core+ |

| GPU | Optional | NVIDIA (CUDA) or Apple Silicon for 3–5× speed |

| Disk | 3 GB free | 5 GB+ |

| OS | Windows 10, macOS 12, Ubuntu 20.04 | Latest |



The Whisper **large-v3** model is used — it gives the best transcription quality across all languages but requires more resources than smaller models. On a modern laptop without GPU, a 30-minute video takes ~10–15 minutes to transcribe.



---



## 🚀 Getting Started (Development)



### 1. Install Node.js

Download from [nodejs.org](https://nodejs.org) — version 18 or higher.



### 2. Clone and install dependencies

```bash

git clone https://github.com/a1lab/a1slice

cd a1slice

npm install

```



### 3. Download whisper.cpp binaries



A1 Slice uses [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — a fast C++ port of Whisper that runs without Python.



You need to place the **pre-built binary** for your platform in `resources/bin/`:



| Platform | Binary name | Where to get it |

|----------|------------|-----------------|

| Windows x64 | `whisper-cli-win-x64.exe` | [Releases](https://github.com/ggerganov/whisper.cpp/releases) → `whisper-bin-win-x64.zip` |

| macOS Apple Silicon | `whisper-cli-mac-arm64` | [Releases](https://github.com/ggerganov/whisper.cpp/releases) → `whisper-bin-osx-arm64.zip` |

| macOS Intel | `whisper-cli-mac-x64` | [Releases](https://github.com/ggerganov/whisper.cpp/releases) → `whisper-bin-osx-x64.zip` |

| Linux x64 | `whisper-cli-linux-x64` | Build from source (see below) |

| Linux arm64 | `whisper-cli-linux-arm64` | Build from source (see below) |



After downloading, place the binary in `resources/bin/` and make it executable on Mac/Linux:

```bash

chmod +x resources/bin/whisper-cli-mac-arm64

```



**Building from source (Linux / custom):**

```bash

git clone https://github.com/ggerganov/whisper.cpp

cd whisper.cpp

make

# Copy the built binary:

cp main /path/to/a1slice/resources/bin/whisper-cli-linux-x64

```



### 4. Get an API key



You need one of:

- **Anthropic Claude** → [console.anthropic.com](https://console.anthropic.com) → API Keys

- **OpenAI GPT-4o** → [platform.openai.com](https://platform.openai.com) → API Keys



The AI is only used to identify which segments to cut — it reads the transcript, not the video. A typical 30-minute video costs **$0.01–0.05** in API credits.



### 5. Run in development mode

```bash

npm run dev

```



---



## 📦 Building a Distributable App



```bash

# Build for current platform

npm run dist



# Build for specific platform

npm run dist:win     # Windows installer (.exe)

npm run dist:mac     # macOS disk image (.dmg)

npm run dist:linux   # Linux AppImage

```



The output is in the `release/` folder.



> **Note for distribution:** You must include the whisper.cpp binary for each target platform in `resources/bin/` before building. The Whisper model (~1.5 GB) is **not** bundled — it downloads automatically on first use and is cached in `~/.a1slice/models/`.



---



## 🔄 How It Works



```

Your video

    │

    ▼

FFmpeg → extract 16kHz mono WAV

    │

    ▼

whisper.cpp (large-v3) → timestamped transcript

    │

    ▼

Claude / GPT-4o → identify best self-contained segments

    │

    ▼

FFmpeg → cut clips + burn subtitles

    │

    ▼

Output folder: 01_Title.mp4, 02_Title.mp4, ...

```



---



## 🗂️ Project Structure



```

a1slice/

├── src/

│   ├── main/           # Electron main process (Node.js)

│   │   ├── main.ts     # Window + IPC handlers

│   │   ├── preload.ts  # Secure bridge to renderer

│   │   ├── whisper.ts  # Model download + transcription

│   │   ├── ffmpeg.ts   # Audio extraction + video cutting

│   │   └── analyzer.ts # LLM integration

│   ├── renderer/       # React UI

│   │   ├── App.tsx

│   │   └── App.css

│   └── shared/

│       └── types.ts    # Shared TypeScript types

├── resources/

│   └── bin/            # whisper.cpp binaries (you provide these)

├── package.json

├── vite.config.ts

└── tsconfig.main.json

```



---



## 🛠️ Tech Stack



| Tool | Purpose |

|------|---------|

| Electron | Desktop app shell |

| React + TypeScript | UI |

| Vite | Fast bundler/dev server |

| whisper.cpp | Local Whisper transcription (no Python) |

| fluent-ffmpeg + ffmpeg-static | Video/audio processing |

| Anthropic SDK / OpenAI SDK | AI segment analysis |

| electron-builder | Cross-platform packaging |



---



## 📄 License



MIT — do whatever you want with it.
