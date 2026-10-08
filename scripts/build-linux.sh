#!/usr/bin/env bash
#
# Build A1 Slice for Linux as an AppImage with whisper CPU + GPU (CUDA) support.
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$REPO_ROOT/dist"
APPDIR="$REPO_ROOT/target/appimage/AppDir"
BIN_DIR="$REPO_ROOT/resources/bin"

cyan()   { printf '\033[0;36m%s\033[0m\n' "$*"; }
green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[0;33m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }

cyan "==> 1. Compiling a1slice in release mode..."
cargo build -p a1slice --release --offline

cyan "==> 2. Checking whisper binaries..."
if [[ ! -f "$BIN_DIR/whisper-cli-linux-x64" ]] || [[ ! -f "$BIN_DIR/whisper-cli-linux-x64-gpu" ]]; then
  if [[ -f "$REPO_ROOT/scripts/build-whisper.sh" ]]; then
    cyan "Building whisper binaries via scripts/build-whisper.sh..."
    bash "$REPO_ROOT/scripts/build-whisper.sh"
  fi
fi

if [[ ! -f "$BIN_DIR/whisper-cli-linux-x64" ]]; then
  red "ERROR: $BIN_DIR/whisper-cli-linux-x64 not found!"
  exit 1
fi

if [[ ! -f "$BIN_DIR/whisper-cli-linux-x64-gpu" ]]; then
  yellow "WARNING: $BIN_DIR/whisper-cli-linux-x64-gpu not found. Whisper GPU will not be bundled."
fi

cyan "==> 3. Finding or fetching appimagetool..."
APPIMAGETOOL=""
if command -v appimagetool >/dev/null 2>&1; then
  APPIMAGETOOL="appimagetool"
elif [[ -x "/tmp/appimagetool-extracted/AppRun" ]]; then
  APPIMAGETOOL="/tmp/appimagetool-extracted/AppRun"
else
  cyan "Downloading appimagetool..."
  curl -fsSL -o /tmp/appimagetool.AppImage "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x /tmp/appimagetool.AppImage
  (cd /tmp && /tmp/appimagetool.AppImage --appimage-extract && mv squashfs-root appimagetool-extracted)
  APPIMAGETOOL="/tmp/appimagetool-extracted/AppRun"
fi

cyan "==> 4. Assembling AppDir structure..."
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$APPDIR/resources/bin"

# Copy binary stripped
cp "$REPO_ROOT/target/release/a1slice" "$APPDIR/usr/bin/a1slice"
strip "$APPDIR/usr/bin/a1slice"

# Copy resources
cp -r "$REPO_ROOT/resources"/* "$APPDIR/resources/"
chmod +x "$APPDIR/resources/bin"/whisper-cli-* 2>/dev/null || true

# Icons & Desktop
cp "$REPO_ROOT/resources/icon.png" "$APPDIR/a1slice.png"
cp "$REPO_ROOT/resources/icon.png" "$APPDIR/.DirIcon"
cp "$REPO_ROOT/resources/icon.png" "$APPDIR/usr/share/icons/hicolor/256x256/apps/a1slice.png"

cat << 'EOF' > "$APPDIR/a1slice.desktop"
[Desktop Entry]
Name=A1 Slice
Comment=Desktop app for working on one video at a time
Exec=a1slice %U
Icon=a1slice
Terminal=false
Type=Application
Categories=AudioVideo;Video;AudioVideoEditing;
StartupWMClass=a1slice
EOF
cp "$APPDIR/a1slice.desktop" "$APPDIR/usr/share/applications/a1slice.desktop"

cat << 'EOF' > "$APPDIR/AppRun"
#!/bin/sh
set -e
HERE="$(dirname "$(readlink -f "${0}")")"
export APPDIR="$HERE"
export PATH="$HERE/usr/bin:$HERE/resources/bin:$PATH"
export LD_LIBRARY_PATH="$HERE/usr/lib:$LD_LIBRARY_PATH"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$HERE/usr/bin/a1slice" "$@"
EOF
chmod +x "$APPDIR/AppRun"

cyan "==> 5. Generating AppImage..."
mkdir -p "$DIST_DIR"
ARCH=x86_64 "$APPIMAGETOOL" -n --comp gzip "$APPDIR" "$DIST_DIR/a1-slice-linux-x64-gzip.AppImage"
ARCH=x86_64 "$APPIMAGETOOL" -n --comp xz "$APPDIR" "$DIST_DIR/a1-slice-linux-x64.AppImage"

green "==> Linux AppImage build complete!"
ls -lh "$DIST_DIR"/*.AppImage
