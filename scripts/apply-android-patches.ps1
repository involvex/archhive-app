# Apply all ArcHive Android gen/ overlays in one step.
# Run after `tauri android init` / when gen/android was regenerated.
# Called by android-regen.ps1 and APK prebuild scripts.

param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android")
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$gen = if ([System.IO.Path]::IsPathRooted($GenAndroid)) {
    $GenAndroid
} else {
    Join-Path $root $GenAndroid
}

if (-not (Test-Path $gen)) {
    Write-Warning "Android project not found at $gen. Run: bun run tauri android init (or bun run android:regen)"
    exit 1
}

$gradlePath = Join-Path $gen "app\build.gradle.kts"
$manifestPath = Join-Path $gen "app\src\main\AndroidManifest.xml"

Write-Host "Applying Android patches under $gen ..."
& (Join-Path $PSScriptRoot "patch-android-lan.ps1") -GradlePath $gradlePath -ManifestPath $manifestPath
if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) {
    throw "patch-android-lan.ps1 failed with exit code $LASTEXITCODE"
}

& (Join-Path $PSScriptRoot "patch-android-ytdlp.ps1") -GenAndroid $gen
if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) {
    throw "patch-android-ytdlp.ps1 failed with exit code $LASTEXITCODE"
}

Write-Host "Android patches applied (LAN + yt-dlp)."
