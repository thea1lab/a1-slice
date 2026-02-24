#!/usr/bin/env bash
#
# Upload local release artifacts to the GitHub release tagged v{version}.
#
# Reads the version from package.json, creates the release if it doesn't
# exist yet, and uploads all matching artifacts (.AppImage, .dmg, .exe).
#
# Requires: gh (GitHub CLI, authenticated)
#
# Usage:
#   ./scripts/upload-release.sh              # upload artifacts from release/
#   ./scripts/upload-release.sh --dist       # upload artifacts from dist/ instead
#   ./scripts/upload-release.sh --overwrite  # replace existing assets
#

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# --- Parse flags ---
SOURCE_DIR="$REPO_ROOT/release"
OVERWRITE=0
for arg in "$@"; do
  case "$arg" in
    --dist)      SOURCE_DIR="$REPO_ROOT/dist" ;;
    --overwrite) OVERWRITE=1 ;;
    -h|--help)
      echo "Usage: $0 [--dist] [--overwrite]"
      echo "  --dist       Upload from dist/ instead of release/"
      echo "  --overwrite  Replace existing assets on the release"
      exit 0
      ;;
    *) echo "Unknown flag: $arg"; echo "Usage: $0 [--dist] [--overwrite]"; exit 1 ;;
  esac
done

# Colors
cyan()   { printf '\033[0;36m%s\033[0m\n' "$*"; }
green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }

# --- Prerequisite checks ---
if ! command -v gh &>/dev/null; then
  red "ERROR: gh (GitHub CLI) is not installed or not on PATH."
  exit 1
fi

if ! gh auth status &>/dev/null; then
  red "ERROR: gh is not authenticated. Run 'gh auth login' first."
  exit 1
fi

# --- Read version ---
VERSION=$(node -p "require('$REPO_ROOT/package.json').version")
TAG="v${VERSION}"

cyan "==> Version: $VERSION (tag: $TAG)"

# --- Find artifacts matching current version ---
ARTIFACTS=()
for pattern in "*${VERSION}*.AppImage" "*${VERSION}*.dmg" "*${VERSION}*.exe"; do
  while IFS= read -r -d '' file; do
    ARTIFACTS+=("$file")
  done < <(find "$SOURCE_DIR" -maxdepth 1 -name "$pattern" -print0 2>/dev/null)
done

if [[ ${#ARTIFACTS[@]} -eq 0 ]]; then
  red "ERROR: No release artifacts for version $VERSION found in $SOURCE_DIR"
  red "  Looked for: *${VERSION}*.AppImage, *${VERSION}*.dmg, *${VERSION}*.exe"
  exit 1
fi

cyan "==> Artifacts to upload:"
for f in "${ARTIFACTS[@]}"; do
  echo "  $(basename "$f") ($(du -h "$f" | cut -f1))"
done

# --- Create release if it doesn't exist ---
if gh release view "$TAG" &>/dev/null; then
  cyan "==> Release $TAG already exists."
else
  cyan "==> Creating release $TAG..."
  gh release create "$TAG" \
    --title "A1 Slice $TAG" \
    --generate-notes
  green "  Release $TAG created."
fi

# --- Upload artifacts ---
cyan "==> Uploading artifacts to release $TAG..."

UPLOAD_FLAGS=()
if [[ "$OVERWRITE" -eq 1 ]]; then
  UPLOAD_FLAGS+=(--clobber)
fi

gh release upload "$TAG" "${ARTIFACTS[@]}" "${UPLOAD_FLAGS[@]}"

green "==> Done! Artifacts uploaded to release $TAG."
echo ""
gh release view "$TAG" --web 2>/dev/null || echo "  View at: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/tag/$TAG"
