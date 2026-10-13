#!/usr/bin/env bash
set -euo pipefail

WHISPER_TAG="v1.8.3"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$REPO_ROOT/resources/bin"
TMP_DIR=$(mktemp -d)

cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

missing=()
for cmd in git cmake; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    missing+=("$cmd")
  fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
  echo "ERROR: Missing required tools: ${missing[*]}"
  echo "On Debian/Ubuntu/Mint:  sudo apt install cmake git build-essential"
  echo "On Fedora:              sudo dnf install cmake git gcc-c++ make"
  echo "On Arch:                sudo pacman -S cmake git base-devel"
  echo "On macOS:               brew install cmake"
  exit 1
fi

echo "Cloning whisper.cpp $WHISPER_TAG..."
git clone --depth 1 --branch "$WHISPER_TAG" https://github.com/ggerganov/whisper.cpp.git "$TMP_DIR/whisper.cpp"

NPROC=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

OS="$(uname -s)"
ARCH="$(uname -m)"

mkdir -p "$BIN_DIR"

# Ninja and MinGW Makefiles write bin/whisper-cli.exe. Visual Studio writes
# bin/Release/whisper-cli.exe.
copy_whisper_exe() {
  local build_dir="$1"
  local dest="$2"
  local built=""
  local candidate
  for candidate in \
    "$build_dir/bin/whisper-cli.exe" \
    "$build_dir/bin/Release/whisper-cli.exe"
  do
    if [[ -f "$candidate" ]]; then
      built="$candidate"
      break
    fi
  done
  if [[ -z "$built" ]]; then
    echo "whisper-cli.exe was not produced in $build_dir" >&2
    exit 1
  fi
  cp "$built" "$dest"
}

copy_mingw_runtime() {
  if ! command -v gcc >/dev/null 2>&1; then
    return
  fi
  local gcc_bin dll
  gcc_bin="$(dirname "$(command -v gcc)")"
  for dll in libgomp-1.dll libwinpthread-1.dll; do
    if [[ -f "$gcc_bin/$dll" ]]; then
      cp "$gcc_bin/$dll" "$BIN_DIR/$dll"
    fi
  done
}

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
    NAME="whisper-cli-linux-x64"
    [ "$ARCH" = "aarch64" ] && NAME="whisper-cli-linux-arm64"

    echo "Building whisper-cli (CPU)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-cpu" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF -DGGML_CUDA=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build-cpu" --config Release \
      --target whisper-cli -j"$NPROC"
    cp "$TMP_DIR/whisper.cpp/build-cpu/bin/whisper-cli" "$BIN_DIR/$NAME"
    chmod +x "$BIN_DIR/$NAME"

    if command -v nvcc >/dev/null 2>&1; then
      echo "Building whisper-cli (GPU, CUDA)..."
      cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-gpu" \
        -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
        -DGGML_METAL=OFF -DGGML_CUDA=ON \
        -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90"
      cmake --build "$TMP_DIR/whisper.cpp/build-gpu" --config Release \
        --target whisper-cli -j"$NPROC"
      cp "$TMP_DIR/whisper.cpp/build-gpu/bin/whisper-cli" "$BIN_DIR/${NAME}-gpu"
      chmod +x "$BIN_DIR/${NAME}-gpu"

      cuda_bin="$(dirname "$(command -v nvcc)")"
      for dir in "$cuda_bin" "$(dirname "$cuda_bin")/lib64" "/usr/local/cuda/lib64"; do
        if [[ -d "$dir" ]]; then
          for so in "$dir"/libcudart.so* "$dir"/libcublas.so* "$dir"/libcublasLt.so*; do
            if [[ -f "$so" ]]; then
              cp -P "$so" "$BIN_DIR/" 2>/dev/null || true
            fi
          done
        fi
      done
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # Git Bash has gcc, not nmake, so the default NMake generator cannot
    # configure. CUDA's nvcc on Windows still needs cl.exe.
    GEN_ARGS=()
    COMPILER_ARGS=()
    if command -v ninja >/dev/null 2>&1; then
      GEN_ARGS=(-G Ninja)
    elif command -v mingw32-make >/dev/null 2>&1; then
      GEN_ARGS=(-G "MinGW Makefiles")
    fi
    if command -v gcc >/dev/null 2>&1 && ! command -v cl >/dev/null 2>&1; then
      COMPILER_ARGS=(
        -DCMAKE_C_COMPILER=gcc
        -DCMAKE_CXX_COMPILER=g++
        -DCMAKE_EXE_LINKER_FLAGS=-static
      )
    fi
    if [[ -n "${VULKAN_SDK:-}" && -d "$VULKAN_SDK/Bin" ]]; then
      export PATH="$VULKAN_SDK/Bin:$PATH"
    fi

    echo "Building whisper-cli (CPU)..."
    cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-cpu" \
      "${GEN_ARGS[@]}" \
      "${COMPILER_ARGS[@]}" \
      -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
      -DGGML_METAL=OFF -DGGML_CUDA=OFF -DGGML_VULKAN=OFF
    cmake --build "$TMP_DIR/whisper.cpp/build-cpu" --config Release \
      --target whisper-cli -j"$NPROC"
    copy_whisper_exe "$TMP_DIR/whisper.cpp/build-cpu" "$BIN_DIR/whisper-cli-win-x64.exe"
    # OpenMP stays dynamic even with -static. Windows loads these from
    # the directory that contains the exe.
    copy_mingw_runtime

    GPU_LABEL=""
    GPU_ARGS=()
    GPU_COMPILER=()
    if command -v nvcc >/dev/null 2>&1 && command -v cl >/dev/null 2>&1; then
      GPU_LABEL="CUDA"
      # nvcc on Windows uses cl as the host compiler. If gcc is earlier on
      # PATH, CMake would compile CUDA with cl and link with ld.
      GPU_COMPILER=(
        -DCMAKE_C_COMPILER=cl
        -DCMAKE_CXX_COMPILER=cl
        -DCMAKE_CUDA_HOST_COMPILER=cl
      )
      GPU_ARGS=(
        -DGGML_METAL=OFF -DGGML_VULKAN=OFF -DGGML_CUDA=ON
        -DCMAKE_CUDA_ARCHITECTURES="75;80;86;89;90"
      )
    elif command -v nvcc >/dev/null 2>&1; then
      echo "Skipping CUDA build: nvcc is installed, but cl.exe is not on PATH."
      echo "Open the x64 Native Tools prompt, or install the Vulkan SDK for a Vulkan build."
    elif command -v glslc >/dev/null 2>&1; then
      GPU_LABEL="Vulkan"
      GPU_ARGS=(-DGGML_METAL=OFF -DGGML_CUDA=OFF -DGGML_VULKAN=ON)
      GPU_COMPILER=("${COMPILER_ARGS[@]}")
    else
      echo "Skipping GPU build. Install the CUDA toolkit plus Visual Studio, or the Vulkan SDK (glslc), then re-run."
    fi

    if [[ -n "$GPU_LABEL" ]]; then
      echo "Building whisper-cli (GPU, $GPU_LABEL)..."
      cmake -S "$TMP_DIR/whisper.cpp" -B "$TMP_DIR/whisper.cpp/build-gpu" \
        "${GEN_ARGS[@]}" \
        "${GPU_COMPILER[@]}" \
        "${GPU_ARGS[@]}" \
        -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF
      cmake --build "$TMP_DIR/whisper.cpp/build-gpu" --config Release \
        --target whisper-cli -j"$NPROC"
      copy_whisper_exe "$TMP_DIR/whisper.cpp/build-gpu" "$BIN_DIR/whisper-cli-win-x64-gpu.exe"
      if [[ "$GPU_LABEL" == "CUDA" ]]; then
        cuda_bin="$(dirname "$(command -v nvcc)")"
        for dir in "$cuda_bin" "$cuda_bin/x64"; do
          for dll in "$dir"/cudart64_*.dll "$dir"/cublas64_*.dll "$dir"/cublasLt64_*.dll; do
            if [[ -f "$dll" ]]; then
              cp "$dll" "$BIN_DIR/"
            fi
          done
        done
      fi
    fi
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

echo "Done: $(ls "$BIN_DIR"/whisper-cli-*)"
