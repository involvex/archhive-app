# Downloads yt-dlp, ffmpeg, and gallery-dl into src-tauri/binaries for sidecar bundling.
param(
    [string]$Arch = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $PSScriptRoot "..\src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$YtDlpUrl = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
$YtDlpOut = Join-Path $BinDir "yt-dlp-$Arch.exe"
Write-Host "Downloading yt-dlp..."
Invoke-WebRequest -Uri $YtDlpUrl -OutFile $YtDlpOut

$FfmpegOut = Join-Path $BinDir "ffmpeg-$Arch.exe"
if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
    Write-Host "Copying ffmpeg from PATH..."
    Copy-Item (Get-Command ffmpeg).Source $FfmpegOut -Force
} else {
    $FfmpegUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
    $ZipPath = Join-Path $env:TEMP "ffmpeg-essentials.zip"
    Write-Host "Downloading ffmpeg..."
    Invoke-WebRequest -Uri $FfmpegUrl -OutFile $ZipPath
    Expand-Archive -Path $ZipPath -DestinationPath $env:TEMP -Force
    $FfmpegExe = Get-ChildItem -Path $env:TEMP -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1
    Copy-Item $FfmpegExe.FullName $FfmpegOut -Force
}

Write-Host "Installing gallery-dl (optional)..."
$GalleryDir = Join-Path $BinDir "gallery-dl-bundle"
$galleryCmd = Get-Command gallery-dl -ErrorAction SilentlyContinue
if ($galleryCmd) {
    Write-Host "gallery-dl already on PATH at $($galleryCmd.Source)"
} else {
    $pipExit = $null
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"

    if (Get-Command py -ErrorAction SilentlyContinue) {
        Write-Host "Running: py -3 -m pip install --target $GalleryDir gallery-dl"
        & py -3 -m pip install --target $GalleryDir gallery-dl
        $pipExit = $LASTEXITCODE
    } elseif (Get-Command python -ErrorAction SilentlyContinue) {
        Write-Host "Running: python -m pip install --target $GalleryDir gallery-dl"
        & python -m pip install --target $GalleryDir gallery-dl
        $pipExit = $LASTEXITCODE
    }

    $ErrorActionPreference = $prevEap

    if ($pipExit -eq 0) {
        @"
@echo off
py -3 "%~dp0gallery-dl-bundle\gallery_dl\__main__.py" %*
"@ | Set-Content (Join-Path $BinDir "gallery-dl-$Arch.exe.cmd")
        Write-Host "gallery-dl installed to $GalleryDir"
    } else {
        Write-Warning "gallery-dl install failed; add to PATH via: py -3 -m pip install gallery-dl"
    }
}

Write-Host "Done. Binaries in $BinDir"
