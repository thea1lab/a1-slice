#!/usr/bin/env bash
set -euo pipefail

WHISPER_TAG="v1.8.3"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$REPO_ROOT/resources/bin"
TMP_DIR=$(mktemp -d)

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "Cloning whisper.cpp $WHISPER_TAG..."
git clone --depth 1 --branch "$WHISPER_TAG" https://github.com/ggerganov/whisper.cpp.git "$TMP_DIR/whisper.cpp"

echo "Building whisper-cli..."
cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build" \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF
cmake --build "$TMP_DIR/whisper.cpp/build" --config Release \
  --target whisper-cli -j"$(nproc || echo 4)"

mkdir -p "$BIN_DIR"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    NAME="whisper-cli-linux-x64"
    [ "$ARCH" = "aarch64" ] && NAME="whisper-cli-linux-arm64"
    cp "$TMP_DIR/whisper.cpp/build/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"
    ;;
  Darwin)
    NAME="whisper-cli-mac-arm64"
    [ "$ARCH" = "x86_64" ] && NAME="whisper-cli-mac-x64"
    cp "$TMP_DIR/whisper.cpp/build/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    cp "$TMP_DIR/whisper.cpp/build/bin/Release/whisper-cli.exe" "$BIN_DIR/whisper-cli-win-x64.exe"
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

echo "Done: $(ls "$BIN_DIR"/whisper-cli-*)"
