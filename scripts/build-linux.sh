#!/usr/bin/env bash
#
# Build A1 Slice for Linux — CPU + GPU (CUDA) whisper-cli binaries, packaged as AppImage.
#
# Requires: docker
# Uses a multi-stage Docker build (nvidia/cuda base) to compile whisper.cpp
# with CUDA support, then packages the Electron app into an AppImage.
# Final installer is placed in dist/.
#
# Usage:
#   ./scripts/build-linux.sh            # normal build (uses cache)
#   ./scripts/build-linux.sh --clean    # force fresh build (removes caches)
#

set -euo pipefail

# --- Parse flags ---
CLEAN=0
for arg in "$@"; do
  case "$arg" in
    --clean|-c) CLEAN=1 ;;
    *) echo "Unknown flag: $arg"; echo "Usage: $0 [--clean]"; exit 1 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"
RELEASE_DIR="$REPO_ROOT/release"
DOCKER_IMAGE="a1slice-builder"

# Colors
cyan()   { printf '\033[0;36m%s\033[0m\n' "$*"; }
green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }

# --- Prerequisite checks ---
cyan "==> Checking prerequisites..."
missing=()
for cmd in docker; do
  if ! command -v "$cmd" &>/dev/null; then
    missing+=("$cmd")
  fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
  red "ERROR: Missing required tools: ${missing[*]}"
  red "Install them and ensure they are on your PATH."
  exit 1
fi
echo "  docker : $(docker --version)"

# Verify Docker daemon is running
if ! docker info &>/dev/null; then
  red "ERROR: Docker daemon is not running. Start Docker and try again."
  exit 1
fi

# --- Clean flag ---
if [[ "$CLEAN" -eq 1 ]]; then
  yellow "==> Clean build requested. Removing caches..."
  docker volume rm a1slice-node-modules a1slice-electron-cache a1slice-eb-cache 2>/dev/null || true
  docker rmi "$DOCKER_IMAGE" 2>/dev/null || true
  if [[ -d "$RELEASE_DIR" ]]; then
    docker run --rm -v "$REPO_ROOT":/build alpine rm -rf /build/release
  fi
  rm -rf "$DIST_DIR"
  green "  Caches cleared."
fi

# --- Build Docker image (compiles whisper CPU + GPU binaries) ---
cyan "==> Building Docker image ($DOCKER_IMAGE)..."
echo "  This compiles whisper.cpp with CPU and CUDA GPU support."
echo "  First run takes a while; subsequent runs use Docker layer cache."
docker build -t "$DOCKER_IMAGE" "$REPO_ROOT"

# --- Clean release dir to avoid mixing old artifacts ---
# Docker runs as root, so release/ may contain root-owned files
if [[ -d "$RELEASE_DIR" ]]; then
  docker run --rm -v "$REPO_ROOT":/build alpine rm -rf /build/release
fi

# --- Run build inside container ---
cyan "==> Running Electron build inside container (target: linux)..."
docker run --rm \
  -v "$REPO_ROOT":/build \
  -v a1slice-node-modules:/build/node_modules \
  -v a1slice-electron-cache:/root/.cache/electron \
  -v a1slice-eb-cache:/root/.cache/electron-builder \
  -e TARGET=linux \
  "$DOCKER_IMAGE"

# --- Copy to dist/ ---
cyan "==> Copying AppImage to dist/..."
mkdir -p "$DIST_DIR"
cp "$RELEASE_DIR"/*.AppImage "$DIST_DIR/" 2>/dev/null || {
  red "ERROR: No .AppImage found in $RELEASE_DIR"
  echo "Contents of $RELEASE_DIR:"
  ls -lh "$RELEASE_DIR/" 2>/dev/null || echo "  (directory does not exist)"
  exit 1
}

echo ""
green "==> Build complete! Output in dist/:"
ls -lh "$DIST_DIR"/*.AppImage
