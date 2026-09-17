# Preflight checks for a patched src-tauri/gen/android tree.
# Exits 0 when ready to build; 1 when gen/android is missing or overlays are incomplete.

param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android"),
    [switch]$Strict
)

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$gen = if ([System.IO.Path]::IsPathRooted($GenAndroid)) {
    $GenAndroid
} else {
    Join-Path $root $GenAndroid
}

$failures = [System.Collections.Generic.List[string]]::new()
$warnings = [System.Collections.Generic.List[string]]::new()

function Add-Fail([string]$msg) { [void]$failures.Add($msg) }
function Add-Warn([string]$msg) { [void]$warnings.Add($msg) }

Write-Host "Validating Android build tree: $gen"

if (-not (Test-Path $gen)) {
    Write-Host "SKIP: gen/android not found. Run: bun run android:regen"
    if ($Strict) {
        exit 1
    }
    exit 0
}

$plugin = Join-Path $gen "app\src\main\java\com\archhive\app\YtDlpPlugin.kt"
$fgService = Join-Path $gen "app\src\main\java\com\archhive\app\DownloadForegroundService.kt"
$resumeWorker = Join-Path $gen "app\src\main\java\com\archhive\app\PendingResumeWorker.kt"
$gradle = Join-Path $gen "app\build.gradle.kts"
$proguard = Join-Path $gen "app\proguard-rules.pro"
$manifest = Join-Path $gen "app\src\main\AndroidManifest.xml"
$binAssets = Join-Path $gen "app\src\main\assets\binaries"

if (-not (Test-Path $plugin)) {
    Add-Fail "Missing YtDlpPlugin.kt (run scripts/apply-android-patches.ps1)"
}
if (-not (Test-Path $fgService)) {
    Add-Fail "Missing DownloadForegroundService.kt"
}
if (-not (Test-Path $resumeWorker)) {
    Add-Fail "Missing PendingResumeWorker.kt"
}

if (-not (Test-Path $gradle)) {
    Add-Fail "Missing app/build.gradle.kts"
} else {
    $gradleText = Get-Content $gradle -Raw
    if ($gradleText -notmatch 'youtubedl-android:library') {
        Add-Fail "build.gradle.kts missing youtubedl-android:library dependency"
    }
    if ($gradleText -notmatch 'youtubedl-android:ffmpeg') {
        Add-Fail "build.gradle.kts missing youtubedl-android:ffmpeg dependency"
    }
    if ($gradleText -notmatch 'work-runtime-ktx') {
        Add-Warn "build.gradle.kts missing work-runtime-ktx (PendingResumeWorker)"
    }
    if ($gradleText -notmatch 'manifestPlaceholders\["usesCleartextTraffic"\]\s*=\s*"true"') {
        Add-Fail "build.gradle.kts missing usesCleartextTraffic=true placeholder (LAN HTTP)"
    }
}

if (-not (Test-Path $proguard)) {
    Add-Warn "Missing proguard-rules.pro"
} else {
    $pg = Get-Content $proguard -Raw
    if ($pg -notmatch 'YtDlpPlugin') {
        Add-Fail "proguard-rules.pro missing YtDlpPlugin keep rules"
    }
}

if (-not (Test-Path $manifest)) {
    Add-Fail "Missing AndroidManifest.xml"
} else {
    $mf = Get-Content $manifest -Raw
    if ($mf -notmatch 'android\.permission\.INTERNET') {
        Add-Fail "AndroidManifest missing INTERNET permission"
    }
    if ($mf -notmatch 'supportsPictureInPicture') {
        Add-Warn "AndroidManifest may be missing PiP support (patch-android-lan)"
    }
    if ($mf -notmatch 'CHANGE_WIFI_MULTICAST_STATE') {
        Add-Warn "AndroidManifest may be missing mDNS CHANGE_WIFI_MULTICAST_STATE"
    }
    if ($mf -notmatch 'DownloadForegroundService') {
        Add-Fail "AndroidManifest missing DownloadForegroundService"
    }
    if ($mf -notmatch 'FOREGROUND_SERVICE_DATA_SYNC') {
        Add-Warn "AndroidManifest may be missing FOREGROUND_SERVICE_DATA_SYNC"
    }
}

if (Test-Path $binAssets) {
    $stale = Get-ChildItem -Path $binAssets -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match 'yt-dlp|gallery-dl|ffmpeg' }
    if ($stale) {
        Add-Fail ("Stale desktop sidecars under assets/binaries: " + ($stale.Name -join ", "))
    }
}

foreach ($w in $warnings) {
    Write-Host "WARN: $w"
}

if ($failures.Count -gt 0) {
    foreach ($f in $failures) {
        Write-Host "FAIL: $f"
    }
    Write-Host "Validation failed ($($failures.Count) error(s)). Fix with: .\scripts\apply-android-patches.ps1"
    exit 1
}

Write-Host "OK: Android build overlays look complete."
exit 0
