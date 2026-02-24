<#
.SYNOPSIS
  Build A1 Slice for Windows with CPU + GPU (CUDA) whisper-cli binaries.

.DESCRIPTION
  Requires: cmake, nvcc (CUDA toolkit), node, npm, Visual Studio (MSVC).
  Clones whisper.cpp v1.8.3 into .cache/whisper.cpp/ (cached between runs).
  Builds CPU and GPU variants, copies CUDA runtime DLLs, packages Electron app.
  Final installer is placed in dist/.

.PARAMETER Clean
  Force a fresh whisper build (removes .cache/whisper.cpp/ build dirs).
#>
param(
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $RepoRoot

$WhisperTag = "v1.8.3"
$CacheDir = Join-Path (Join-Path $RepoRoot ".cache") "whisper.cpp"
$BinDir = Join-Path (Join-Path $RepoRoot "resources") "bin"
$DistDir = Join-Path $RepoRoot "dist"
$CpuBinary = Join-Path $BinDir "whisper-cli-win-x64.exe"
$GpuBinary = Join-Path $BinDir "whisper-cli-win-x64-gpu.exe"
$CudaArchitectures = "75;80;86;89;90"

# --- Prerequisite checks ---
Write-Host "==> Checking prerequisites..." -ForegroundColor Cyan
$missing = @()
foreach ($cmd in @("cmake", "nvcc", "node", "npm")) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        $missing += $cmd
    }
}
if ($missing.Count -gt 0) {
    Write-Host "ERROR: Missing required tools: $($missing -join ', ')" -ForegroundColor Red
    Write-Host "Install them and ensure they are on your PATH." -ForegroundColor Red
    exit 1
}
Write-Host "  cmake : $(cmake --version | Select-Object -First 1)"
Write-Host "  nvcc  : $(nvcc --version | Select-String 'release' | ForEach-Object { $_.Line.Trim() })"
Write-Host "  node  : $(node --version)"
Write-Host "  npm   : $(npm --version)"

# --- Set up MSVC environment via vcvarsall.bat ---
if (-not (Get-Command "cl" -ErrorAction SilentlyContinue)) {
    Write-Host "==> Setting up MSVC environment..." -ForegroundColor Cyan
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) {
        Write-Host "ERROR: vswhere not found. Install Visual Studio Build Tools." -ForegroundColor Red
        exit 1
    }
    $vsPath = & $vswhere -latest -products * -property installationPath
    if (-not $vsPath) {
        Write-Host "ERROR: No Visual Studio or Build Tools installation found." -ForegroundColor Red
        exit 1
    }
    Write-Host "  Found: $vsPath"
    $vcvarsall = Join-Path (Join-Path $vsPath "VC") "Auxiliary\Build\vcvarsall.bat"
    if (-not (Test-Path $vcvarsall)) {
        Write-Host "ERROR: vcvarsall.bat not found at $vcvarsall" -ForegroundColor Red
        exit 1
    }
    # Source vcvarsall.bat and import env vars into PowerShell
    $output = cmd /c "`"$vcvarsall`" x64 >nul 2>&1 && set"
    foreach ($line in $output) {
        if ($line -match '^([^=]+)=(.*)$') {
            [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
    $ErrorActionPreference = "Continue"
    $clVersion = (cl 2>&1 | ForEach-Object { "$_" } | Select-Object -First 1) -replace '.*Compiler Version\s+', 'MSVC '
    $ErrorActionPreference = "Stop"
    Write-Host "  cl    : $clVersion"
} else {
    $ErrorActionPreference = "Continue"
    $clVersion = (cl 2>&1 | ForEach-Object { "$_" } | Select-Object -First 1) -replace '.*Compiler Version\s+', 'MSVC '
    $ErrorActionPreference = "Stop"
    Write-Host "  cl    : $clVersion"
}

# Detect cmake generator — prefer Ninja, fall back to NMake
if (Get-Command "ninja" -ErrorAction SilentlyContinue) {
    $cmakeGenerator = "Ninja"
} else {
    $cmakeGenerator = "NMake Makefiles"
}
Write-Host "  generator: $cmakeGenerator"

# --- Clean flag ---
if ($Clean) {
    Write-Host "==> Clean build requested. Removing cached builds..." -ForegroundColor Yellow
    $cpuBuildDir = Join-Path $CacheDir "build-cpu"
    $gpuBuildDir = Join-Path $CacheDir "build-gpu"
    if (Test-Path $cpuBuildDir) { Remove-Item -Recurse -Force $cpuBuildDir }
    if (Test-Path $gpuBuildDir) { Remove-Item -Recurse -Force $gpuBuildDir }
    if (Test-Path $CpuBinary) { Remove-Item -Force $CpuBinary }
    if (Test-Path $GpuBinary) { Remove-Item -Force $GpuBinary }
}

# --- Clone whisper.cpp (if not cached) ---
if (-not (Test-Path (Join-Path $CacheDir "CMakeLists.txt"))) {
    Write-Host "==> Cloning whisper.cpp $WhisperTag into .cache/whisper.cpp/..." -ForegroundColor Cyan
    New-Item -ItemType Directory -Force -Path (Split-Path $CacheDir) | Out-Null
    git clone --depth 1 --branch $WhisperTag https://github.com/ggerganov/whisper.cpp.git $CacheDir
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: git clone failed" -ForegroundColor Red; exit 1 }
} else {
    Write-Host "==> Using cached whisper.cpp in .cache/whisper.cpp/" -ForegroundColor Cyan
}

# --- Build whisper binaries (skip if both already exist) ---
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$needBuild = (-not (Test-Path $CpuBinary)) -or (-not (Test-Path $GpuBinary))

if ($needBuild) {
    # CPU build
    if (-not (Test-Path $CpuBinary)) {
        Write-Host "==> Building whisper-cli (CPU)..." -ForegroundColor Cyan
        $cpuBuildDir = Join-Path $CacheDir "build-cpu"
        cmake -S $CacheDir -B $cpuBuildDir -G "$cmakeGenerator" `
            -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF `
            -DGGML_METAL=OFF -DGGML_CUDA=OFF
        if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: cmake configure (CPU) failed" -ForegroundColor Red; exit 1 }

        cmake --build $cpuBuildDir --config Release --target whisper-cli --parallel
        if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: cmake build (CPU) failed" -ForegroundColor Red; exit 1 }

        $cpuExe = Join-Path (Join-Path $cpuBuildDir "bin") "whisper-cli.exe"
        if (-not (Test-Path $cpuExe)) {
            $cpuExe = Join-Path (Join-Path (Join-Path $cpuBuildDir "bin") "Release") "whisper-cli.exe"
        }
        Copy-Item $cpuExe $CpuBinary
        Write-Host "  -> $CpuBinary" -ForegroundColor Green
    } else {
        Write-Host "==> CPU binary already exists, skipping build." -ForegroundColor Cyan
    }

    # GPU build
    if (-not (Test-Path $GpuBinary)) {
        Write-Host "==> Building whisper-cli (GPU, CUDA)..." -ForegroundColor Cyan
        $gpuBuildDir = Join-Path $CacheDir "build-gpu"
        cmake -S $CacheDir -B $gpuBuildDir -G "$cmakeGenerator" `
            -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF `
            -DGGML_METAL=OFF -DGGML_CUDA=ON `
            -DCMAKE_CUDA_ARCHITECTURES="$CudaArchitectures" `
            -DCMAKE_CUDA_FLAGS="--allow-unsupported-compiler"
        if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: cmake configure (GPU) failed" -ForegroundColor Red; exit 1 }

        cmake --build $gpuBuildDir --config Release --target whisper-cli --parallel
        if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: cmake build (GPU) failed" -ForegroundColor Red; exit 1 }

        $gpuExe = Join-Path (Join-Path $gpuBuildDir "bin") "whisper-cli.exe"
        if (-not (Test-Path $gpuExe)) {
            $gpuExe = Join-Path (Join-Path (Join-Path $gpuBuildDir "bin") "Release") "whisper-cli.exe"
        }
        Copy-Item $gpuExe $GpuBinary
        Write-Host "  -> $GpuBinary" -ForegroundColor Green
    } else {
        Write-Host "==> GPU binary already exists, skipping build." -ForegroundColor Cyan
    }

    # Copy CUDA runtime DLLs
    Write-Host "==> Copying CUDA runtime DLLs..." -ForegroundColor Cyan
    $cudaPath = $env:CUDA_PATH
    if (-not $cudaPath) {
        Write-Host "ERROR: CUDA_PATH environment variable not set" -ForegroundColor Red
        exit 1
    }
    $cudaBinDir = Join-Path $cudaPath "bin"
    $dllPatterns = @("cublas64_*.dll", "cublasLt64_*.dll", "cudart64_*.dll")
    foreach ($pattern in $dllPatterns) {
        $dlls = Get-ChildItem -Path $cudaBinDir -Filter $pattern -ErrorAction SilentlyContinue
        if ($dlls.Count -eq 0) {
            Write-Host "  WARNING: No files matching $pattern in $cudaBinDir" -ForegroundColor Yellow
        }
        foreach ($dll in $dlls) {
            Copy-Item $dll.FullName (Join-Path $BinDir $dll.Name) -Force
            Write-Host "  -> $($dll.Name)" -ForegroundColor Green
        }
    }
} else {
    Write-Host "==> Both whisper binaries already exist, skipping build." -ForegroundColor Cyan
}

# --- Build Electron app ---
Write-Host "==> Installing Node.js dependencies..." -ForegroundColor Cyan
npm ci
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: npm ci failed" -ForegroundColor Red; exit 1 }

Write-Host "==> Building and packaging Windows installer..." -ForegroundColor Cyan
npm run dist:win -- --publish never
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: npm run dist:win failed" -ForegroundColor Red; exit 1 }

# --- Copy to dist/ ---
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
$installers = Get-ChildItem -Path (Join-Path $RepoRoot "release") -Filter "*.exe"
foreach ($installer in $installers) {
    Copy-Item $installer.FullName (Join-Path $DistDir $installer.Name) -Force
    Write-Host "  -> dist/$($installer.Name)" -ForegroundColor Green
}

Write-Host ""
Write-Host "==> Build complete! Output in dist/:" -ForegroundColor Green
Get-ChildItem $DistDir | Format-Table Name, Length -AutoSize
