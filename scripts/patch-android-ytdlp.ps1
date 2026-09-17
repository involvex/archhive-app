# Patches generated Android project with YtDlpPlugin, youtubedl-android deps, and ProGuard keeps.
# Run after `tauri android init` / android:regen (gen/android is gitignored).
#
# Version pin: change $YoutubeDlAndroidVersion below when upgrading.
# Then: bun run android:patches (or android:regen), smoke YtDlpPlugin execute + FFmpeg.init
# on device/emulator, and update docs/mobile-android.md if the pin changes.

param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android"),
    [string]$Overlays = (Join-Path $PSScriptRoot "..\src-tauri\android-overlays")
)

$ErrorActionPreference = "Stop"

# Pin both library + ffmpeg AARs to the same release. Bump together only.
$YoutubeDlAndroidVersion = "0.18.1"
# WorkManager for PendingResumeWorker (download resume after process death / network restore).
$WorkRuntimeKtxVersion = "2.9.1"

$gradlePath = Join-Path $GenAndroid "app\build.gradle.kts"
$proguardPath = Join-Path $GenAndroid "app\proguard-rules.pro"
$pluginDestDir = Join-Path $GenAndroid "app\src\main\java\com\archhive\app"
$pluginDest = Join-Path $pluginDestDir "YtDlpPlugin.kt"
$pluginSrc = Join-Path $Overlays "YtDlpPlugin.kt"
$proguardOverlay = Join-Path $Overlays "proguard-ytdlp.pro"
$fgServiceSrc = Join-Path $Overlays "DownloadForegroundService.kt"
$resumeWorkerSrc = Join-Path $Overlays "PendingResumeWorker.kt"

if (-not (Test-Path $gradlePath)) {
    Write-Warning "Android project not found at $GenAndroid. Run: bun run tauri android init"
    exit 1
}

if (-not (Test-Path $pluginSrc)) {
    throw "Missing overlay: $pluginSrc"
}

New-Item -ItemType Directory -Force -Path $pluginDestDir | Out-Null
Copy-Item -Path $pluginSrc -Destination $pluginDest -Force
Write-Host "Installed YtDlpPlugin.kt -> $pluginDest"

if (Test-Path $fgServiceSrc) {
    Copy-Item -Path $fgServiceSrc -Destination (Join-Path $pluginDestDir "DownloadForegroundService.kt") -Force
    Write-Host "Installed DownloadForegroundService.kt"
}
if (Test-Path $resumeWorkerSrc) {
    Copy-Item -Path $resumeWorkerSrc -Destination (Join-Path $pluginDestDir "PendingResumeWorker.kt") -Force
    Write-Host "Installed PendingResumeWorker.kt"
}

# Ensure youtubedl-android + WorkManager dependencies
$gradle = Get-Content $gradlePath -Raw
$changed = $false

if ($gradle -notmatch 'youtubedl-android:library') {
    $deps = @"
    // youtubedl-android: bundles Python + yt-dlp for ARM64/ARMv7 Android (pin: $YoutubeDlAndroidVersion)
    implementation("io.github.junkfood02.youtubedl-android:library:$YoutubeDlAndroidVersion")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:$YoutubeDlAndroidVersion")
    // WorkManager: PendingResumeWorker wakes app to re-queue downloads
    implementation("androidx.work:work-runtime-ktx:$WorkRuntimeKtxVersion")
"@
    if ($gradle -match '(?m)^dependencies \{') {
        $gradle = $gradle -replace '(?m)^(dependencies \{)', "`$1`r`n$deps"
        $changed = $true
        Write-Host "Added youtubedl-android $YoutubeDlAndroidVersion + WorkManager deps to build.gradle.kts"
    } else {
        Write-Warning "Could not find dependencies block in $gradlePath"
    }
} elseif ($gradle -notmatch 'work-runtime-ktx') {
    $wmDep = @"
    implementation("androidx.work:work-runtime-ktx:$WorkRuntimeKtxVersion")
"@
    if ($gradle -match '(?m)^dependencies \{') {
        $gradle = $gradle -replace '(?m)^(dependencies \{)', "`$1`r`n$wmDep"
        $changed = $true
        Write-Host "Added WorkManager dependency to build.gradle.kts"
    }
}

# Prefer ABI filters that match youtubedl-android native libs
if ($gradle -notmatch 'youtubedl-android ships native libs') {
    if ($gradle -match '(?s)ndk\s*\{[^}]*\}') {
        $ndkBlock = @'
        ndk {
            // youtubedl-android ships native libs for these ABIs
            abiFilters += listOf("arm64-v8a", "armeabi-v7a")
        }
'@
        $gradle = [regex]::Replace($gradle, '(?s)(\s*)ndk\s*\{[^}]*\}', "`n$ndkBlock")
        $changed = $true
        Write-Host "Set abiFilters for arm64-v8a / armeabi-v7a"
    }
}

if ($changed) {
    Set-Content -Path $gradlePath -Value $gradle -NoNewline
} else {
    Write-Host "youtubedl-android Gradle deps already present (pin $YoutubeDlAndroidVersion)."
}

# Merge ProGuard keep rules
if (-not (Test-Path $proguardOverlay)) {
    throw "Missing overlay: $proguardOverlay"
}
$keepMarker = "DownloadForegroundService"
$overlayText = Get-Content $proguardOverlay -Raw
if (Test-Path $proguardPath) {
    $existing = Get-Content $proguardPath -Raw
    if ($existing -notmatch [regex]::Escape($keepMarker)) {
        # Replace legacy YtDlp-only keeps with full overlay (includes FG service + WorkManager).
        if ($existing -match 'YtDlpPlugin') {
            $without = [regex]::Replace($existing, '(?s)# ArcHive Android.*?dontwarn com\.yausername\.\*\*', '').TrimEnd()
            $merged = $without + "`r`n`r`n" + $overlayText.TrimEnd() + "`r`n"
        } else {
            $merged = $existing.TrimEnd() + "`r`n`r`n" + $overlayText.TrimEnd() + "`r`n"
        }
        Set-Content -Path $proguardPath -Value $merged -NoNewline
        Write-Host "Updated ProGuard keep rules (YtDlp + FG + WorkManager) in $proguardPath"
    } else {
        Write-Host "ProGuard YtDlp/FG keep rules already present."
    }
} else {
    Set-Content -Path $proguardPath -Value $overlayText -NoNewline
    Write-Host "Created $proguardPath with YtDlp keep rules"
}

Write-Host "Android YtDlp overlay patch complete (youtubedl-android $YoutubeDlAndroidVersion)."

# Remove stale Linux BtbN ffmpeg/ffprobe assets left from older tauri.android.conf.json
# resource mappings (they cannot run on Android/bionic and inflate the APK by ~260MB).
$staleBinaries = Join-Path $GenAndroid "app\src\main\assets\binaries"
if (Test-Path $staleBinaries) {
    Remove-Item -Recurse -Force $staleBinaries
    Write-Host "Removed stale assets/binaries (Linux ffmpeg sidecars)."
}

# Also apply LAN cleartext + PiP manifest patches when gen/android exists.
& (Join-Path $PSScriptRoot "patch-android-lan.ps1")
