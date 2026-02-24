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

NPROC=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

OS="$(uname -s)"
ARCH="$(uname -m)"

mkdir -p "$BIN_DIR"

case "$OS" in
  Darwin)
    # macOS: build CPU + GPU (Metal) variants
    NAME="whisper-cli-mac-arm64"
    [ "$ARCH" = "x86_64" ] && NAME="whisper-cli-mac-x64"

    echo "Building whisper-cli (CPU)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-cpu" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DGGML_METAL=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build-cpu" --config Release \
      --target whisper-cli -j"$NPROC"
    cp "$TMP_DIR/whisper.cpp/build-cpu/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"

    echo "Building whisper-cli (GPU, Metal)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-gpu" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build-gpu" --config Release \
      --target whisper-cli -j"$NPROC"
    cp "$TMP_DIR/whisper.cpp/build-gpu/bin/whisper-cli" "$BIN_DIR/${NAME}-gpu"
    chmod +x "$BIN_DIR/${NAME}-gpu"
    ;;
  Linux)
    # Linux: CPU only
    NAME="whisper-cli-linux-x64"
    [ "$ARCH" = "aarch64" ] && NAME="whisper-cli-linux-arm64"

    echo "Building whisper-cli (CPU)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF -DGGML_CUDA=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build" --config Release \
      --target whisper-cli -j"$NPROC"
    cp "$TMP_DIR/whisper.cpp/build/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # Windows: CPU only
    echo "Building whisper-cli (CPU)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF -DGGML_CUDA=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build" --config Release \
      --target whisper-cli -j"$NPROC"
    cp "$TMP_DIR/whisper.cpp/build/bin/Release/whisper-cli.exe" "$BIN_DIR/whisper-cli-win-x64.exe"
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

echo "Done: $(ls "$BIN_DIR"/whisper-cli-*)"
