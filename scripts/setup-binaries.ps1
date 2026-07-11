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

$FfmpegUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"
$ZipPath = Join-Path $env:TEMP "ffmpeg-essentials.zip"
Write-Host "Downloading ffmpeg..."
Invoke-WebRequest -Uri $FfmpegUrl -OutFile $ZipPath
Expand-Archive -Path $ZipPath -DestinationPath $env:TEMP -Force
$FfmpegExe = Get-ChildItem -Path $env:TEMP -Recurse -Filter "ffmpeg.exe" | Select-Object -First 1
Copy-Item $FfmpegExe.FullName (Join-Path $BinDir "ffmpeg-$Arch.exe") -Force

Write-Host "Installing gallery-dl via pip into local folder..."
$GalleryDir = Join-Path $BinDir "gallery-dl-bundle"
python -m pip install --target $GalleryDir gallery-dl 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Warning "gallery-dl pip install failed; install gallery-dl on PATH manually"
} else {
    @"
@echo off
python "%~dp0gallery-dl-bundle\gallery_dl\__main__.py" %*
"@ | Set-Content (Join-Path $BinDir "gallery-dl-$Arch.exe.cmd")
}

Write-Host "Done. Binaries in $BinDir"
