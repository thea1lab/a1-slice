#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-}"

if [[ "$TARGET" != "linux" && "$TARGET" != "win" ]]; then
  echo "Usage: $0 <linux|win>"
  echo "  linux  → AppImage with CPU + GPU (CUDA) whisper binaries"
  echo "  win    → Windows .exe with CPU-only whisper (cross-compiled via Wine)"
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building Docker image (a1slice-builder)..."
docker build -t a1slice-builder "$REPO_ROOT"

echo "==> Running build for target: $TARGET"
docker run --rm \
  -v "$REPO_ROOT":/build \
  -v a1slice-node-modules:/build/node_modules \
  -v a1slice-electron-cache:/root/.cache/electron \
  -v a1slice-eb-cache:/root/.cache/electron-builder \
  -e TARGET="$TARGET" \
  a1slice-builder

echo "==> Build complete. Output in $REPO_ROOT/release/"
ls -lh "$REPO_ROOT/release/"

# Copy final artifacts to dist/
mkdir -p "$REPO_ROOT/dist"
if [[ "$TARGET" == "linux" ]]; then
  cp "$REPO_ROOT"/release/*.AppImage "$REPO_ROOT/dist/"
elif [[ "$TARGET" == "win" ]]; then
  cp "$REPO_ROOT"/release/*.exe "$REPO_ROOT/dist/"
fi
echo "==> Copied to $REPO_ROOT/dist/"
ls -lh "$REPO_ROOT/dist/"
