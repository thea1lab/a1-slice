#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-linux}"

echo "==> Target: $TARGET"

# Copy CPU whisper binary
echo "==> Copying whisper-cli-linux-x64 (CPU)..."
mkdir -p /build/resources/bin
cp /whisper-cache/whisper-cli-linux-x64 /build/resources/bin/

if [[ "$TARGET" == "linux" ]]; then
  # Linux: include GPU binary in AppImage
  echo "==> Copying whisper-cli-linux-x64-gpu (CUDA)..."
  cp /whisper-cache/whisper-cli-linux-x64-gpu /build/resources/bin/
fi

# Install dependencies
echo "==> Running npm ci..."
cd /build
npm ci

if [[ "$TARGET" == "win" ]]; then
  # Replace Linux ffmpeg binary with Windows one for cross-compile packaging
  FFMPEG_STATIC_VERSION="$(node -e "console.log(require('./node_modules/ffmpeg-static/package.json').version)")"
  FFMPEG_URL="https://github.com/eugeneware/ffmpeg-static/releases/download/b${FFMPEG_STATIC_VERSION}/ffmpeg-win32-x64.gz"
  echo "==> Downloading Windows ffmpeg from $FFMPEG_URL..."
  curl -fsSL "$FFMPEG_URL" | gunzip > /build/node_modules/ffmpeg-static/ffmpeg
  chmod +x /build/node_modules/ffmpeg-static/ffmpeg
fi

# Build and package
echo "==> Running npm run dist:$TARGET..."
npm run "dist:$TARGET"

echo "==> Done!"
