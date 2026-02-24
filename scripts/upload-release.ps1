#
# Upload the release artifact for the current version to GitHub.
#
# Reads the version from package.json, finds the matching artifact in dist\,
# creates the GitHub release if needed, and uploads it.
#
# Requires: gh (GitHub CLI, authenticated), node
#
# Usage:
#   .\scripts\upload-release.ps1              # upload from dist\
#   .\scripts\upload-release.ps1 -Overwrite   # replace existing asset
#

param(
    [switch]$Overwrite,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
}

if ($Help) {
    Write-Host "Usage: .\scripts\upload-release.ps1 [-Overwrite]"
    Write-Host "  -Overwrite   Replace existing asset on the release"
    exit 0
}

# --- Helpers ---
function Write-Cyan   { param($Msg) Write-Host $Msg -ForegroundColor Cyan }
function Write-Green  { param($Msg) Write-Host $Msg -ForegroundColor Green }
function Write-Red    { param($Msg) Write-Host $Msg -ForegroundColor Red }

$DistDir = Join-Path $RepoRoot "dist"

# --- Prerequisite checks ---
if (-not (Get-Command "gh" -ErrorAction SilentlyContinue)) {
    Write-Red "ERROR: gh (GitHub CLI) is not installed or not on PATH."
    exit 1
}

$authCheck = gh auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Red "ERROR: gh is not authenticated. Run 'gh auth login' first."
    exit 1
}

# --- Read version ---
$Version = node -p "require('./package.json').version"
if ($LASTEXITCODE -ne 0) {
    Write-Red "ERROR: Failed to read version from package.json"
    exit 1
}
$Tag = "v$Version"

Write-Cyan "==> Version: $Version (tag: $Tag)"

# --- Find artifact matching current version in dist/ ---
$Artifacts = @()
foreach ($pattern in @("*$Version*.AppImage", "*$Version*.dmg", "*$Version*.exe")) {
    $found = Get-ChildItem -Path $DistDir -Filter $pattern -File -ErrorAction SilentlyContinue
    if ($found) {
        $Artifacts += $found
    }
}

if ($Artifacts.Count -eq 0) {
    Write-Red "ERROR: No artifacts for version $Version found in $DistDir"
    Write-Red "  Looked for: *$Version*.AppImage, *$Version*.dmg, *$Version*.exe"
    exit 1
}

Write-Cyan "==> Artifacts to upload:"
foreach ($f in $Artifacts) {
    $size = "{0:N1} MB" -f ($f.Length / 1MB)
    Write-Host "  $($f.Name) ($size)"
}

# --- Create release if it doesn't exist ---
gh release view $Tag 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Cyan "==> Release $Tag already exists."
} else {
    Write-Cyan "==> Creating release $Tag..."
    gh release create $Tag --title "A1 Slice $Tag" --generate-notes
    if ($LASTEXITCODE -ne 0) {
        Write-Red "ERROR: Failed to create release $Tag"
        exit 1
    }
    Write-Green "  Release $Tag created."
}

# --- Upload artifacts ---
Write-Cyan "==> Uploading artifacts to release $Tag..."

$uploadArgs = @("release", "upload", $Tag)
foreach ($f in $Artifacts) {
    $uploadArgs += $f.FullName
}
if ($Overwrite) {
    $uploadArgs += "--clobber"
}

& gh @uploadArgs
if ($LASTEXITCODE -ne 0) {
    Write-Red "ERROR: Failed to upload artifacts"
    exit 1
}

Write-Green "==> Done! Artifacts uploaded to release $Tag."
Write-Host ""
gh release view $Tag --web 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    $repo = gh repo view --json nameWithOwner -q ".nameWithOwner"
    Write-Host "  View at: https://github.com/$repo/releases/tag/$Tag"
}
