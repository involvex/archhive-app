# Downloads yt-dlp, ffmpeg, and gallery-dl into src-tauri/binaries for sidecar bundling.
param(
    [string]$Arch = "x86_64-pc-windows-msvc",
    [switch]$IncludeAndroid
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
if ($env:CI -eq "true") {
    Write-Host "Skipping gallery-dl pip bundle on CI (not bundled as a Tauri sidecar)."
} else {
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
}

if ($IncludeAndroid) {
    Write-Host "Downloading Android ARM64 ffmpeg/ffprobe..."
    $AndroidFfmpegUrl = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz"
    # Expand-Archive only supports .zip; the Linux ARM64 build is .tar.xz,
    # so extract with the tar.exe bundled in Windows 10 1803+.
    $AndroidStage = Join-Path $env:TEMP "ffmpeg-android-archhive"
    $AndroidTarPath = Join-Path $AndroidStage "ffmpeg-android.tar.xz"
    New-Item -ItemType Directory -Force -Path $AndroidStage | Out-Null
    Invoke-WebRequest -Uri $AndroidFfmpegUrl -OutFile $AndroidTarPath
    $tarCmd = Get-Command tar -ErrorAction SilentlyContinue
    if (-not $tarCmd) {
        throw "The 'tar' command was not found. It ships with Windows 10 1803 and later - please update Windows or extract $AndroidTarPath manually."
    }
    Write-Host "Extracting Android ffmpeg (tar.xz)..."
    & $tarCmd.Source -xf $AndroidTarPath -C $AndroidStage
    if ($LASTEXITCODE -ne 0) {
        throw "tar extraction failed with exit code $LASTEXITCODE"
    }
    $AndroidFfmpegDir = Get-ChildItem -Path $AndroidStage -Directory -Filter "ffmpeg-master-latest-linuxarm64-gpl" | Select-Object -First 1
    if ($AndroidFfmpegDir) {
        $AndroidBinDir = Join-Path $AndroidFfmpegDir.FullName "bin"
        $FfmpegAndroid = Join-Path $BinDir "ffmpeg-aarch64-linux-android"
        $FfprobeAndroid = Join-Path $BinDir "ffprobe-aarch64-linux-android"
        Copy-Item (Join-Path $AndroidBinDir "ffmpeg") $FfmpegAndroid -Force
        Copy-Item (Join-Path $AndroidBinDir "ffprobe") $FfprobeAndroid -Force
        Write-Host "Android ffmpeg/ffprobe installed to $BinDir"
        # Drop the ~300 MB staging dir (tarball + extracted tree).
        Remove-Item -Recurse -Force $AndroidStage
    } else {
        Write-Warning "Android ffmpeg download failed"
    }
}

Write-Host "Done. Binaries in $BinDir"
