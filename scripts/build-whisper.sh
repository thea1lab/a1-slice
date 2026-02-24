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

# --- CPU build (GPU explicitly disabled) ---
echo "Building whisper-cli (CPU)..."
cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-cpu" \
  -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
  -DGGML_METAL=OFF -DGGML_CUDA=OFF
cmake --build "$TMP_DIR/whisper.cpp/build-cpu" --config Release \
  --target whisper-cli -j"$NPROC"

# --- GPU build ---
echo "Building whisper-cli (GPU)..."
GPU_FLAGS=()
case "$OS" in
  Darwin)
    # Metal is auto-enabled on macOS — no extra flags needed
    ;;
  Linux)
    if command -v nvcc &>/dev/null; then
      GPU_FLAGS+=(-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90")
    else
      echo "CUDA toolkit not found — skipping GPU build on Linux"
      GPU_FLAGS=("SKIP")
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    if command -v nvcc &>/dev/null; then
      GPU_FLAGS+=(-DGGML_CUDA=ON -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90")
    else
      echo "CUDA toolkit not found — skipping GPU build on Windows"
      GPU_FLAGS=("SKIP")
    fi
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

GPU_BUILT=false
if [[ "${GPU_FLAGS[0]:-}" != "SKIP" ]]; then
  cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-gpu" \
    -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
    "${GPU_FLAGS[@]}"
  cmake --build "$TMP_DIR/whisper.cpp/build-gpu" --config Release \
    --target whisper-cli -j"$NPROC"
  GPU_BUILT=true
fi

# --- Copy binaries ---
mkdir -p "$BIN_DIR"

case "$OS" in
  Linux)
    NAME="whisper-cli-linux-x64"
    [ "$ARCH" = "aarch64" ] && NAME="whisper-cli-linux-arm64"
    cp "$TMP_DIR/whisper.cpp/build-cpu/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"
    if $GPU_BUILT; then
      cp "$TMP_DIR/whisper.cpp/build-gpu/bin/whisper-cli" "$BIN_DIR/${NAME}-gpu"
      chmod +x "$BIN_DIR/${NAME}-gpu"
    fi
    ;;
  Darwin)
    NAME="whisper-cli-mac-arm64"
    [ "$ARCH" = "x86_64" ] && NAME="whisper-cli-mac-x64"
    cp "$TMP_DIR/whisper.cpp/build-cpu/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"
    if $GPU_BUILT; then
      cp "$TMP_DIR/whisper.cpp/build-gpu/bin/whisper-cli" "$BIN_DIR/${NAME}-gpu"
      chmod +x "$BIN_DIR/${NAME}-gpu"
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    cp "$TMP_DIR/whisper.cpp/build-cpu/bin/Release/whisper-cli.exe" "$BIN_DIR/whisper-cli-win-x64.exe"
    if $GPU_BUILT; then
      cp "$TMP_DIR/whisper.cpp/build-gpu/bin/Release/whisper-cli.exe" "$BIN_DIR/whisper-cli-win-x64-gpu.exe"
    fi
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

echo "Done: $(ls "$BIN_DIR"/whisper-cli-*)"
