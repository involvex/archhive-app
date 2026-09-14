# Patches generated Android project with YtDlpPlugin, youtubedl-android deps, and ProGuard keeps.
# Run after `tauri android init` / android:regen (gen/android is gitignored).

param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android"),
    [string]$Overlays = (Join-Path $PSScriptRoot "..\src-tauri\android-overlays")
)

$ErrorActionPreference = "Stop"

$gradlePath = Join-Path $GenAndroid "app\build.gradle.kts"
$proguardPath = Join-Path $GenAndroid "app\proguard-rules.pro"
$pluginDestDir = Join-Path $GenAndroid "app\src\main\java\com\archhive\app"
$pluginDest = Join-Path $pluginDestDir "YtDlpPlugin.kt"
$pluginSrc = Join-Path $Overlays "YtDlpPlugin.kt"
$proguardOverlay = Join-Path $Overlays "proguard-ytdlp.pro"

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

# Ensure youtubedl-android dependencies
$gradle = Get-Content $gradlePath -Raw
$changed = $false

if ($gradle -notmatch 'youtubedl-android:library') {
    $deps = @'
    // youtubedl-android: bundles Python + yt-dlp for ARM64/ARMv7 Android
    implementation("io.github.junkfood02.youtubedl-android:library:0.18.1")
    implementation("io.github.junkfood02.youtubedl-android:ffmpeg:0.18.1")
'@
    if ($gradle -match '(?m)^dependencies \{') {
        $gradle = $gradle -replace '(?m)^(dependencies \{)', "`$1`r`n$deps"
        $changed = $true
        Write-Host "Added youtubedl-android dependencies to build.gradle.kts"
    } else {
        Write-Warning "Could not find dependencies block in $gradlePath"
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
    Write-Host "youtubedl-android Gradle deps already present."
}

# Merge ProGuard keep rules
if (-not (Test-Path $proguardOverlay)) {
    throw "Missing overlay: $proguardOverlay"
}
$keepMarker = "YtDlpPlugin"
$overlayText = Get-Content $proguardOverlay -Raw
if (Test-Path $proguardPath) {
    $existing = Get-Content $proguardPath -Raw
    if ($existing -notmatch [regex]::Escape($keepMarker)) {
        $merged = $existing.TrimEnd() + "`r`n`r`n" + $overlayText.TrimEnd() + "`r`n"
        Set-Content -Path $proguardPath -Value $merged -NoNewline
        Write-Host "Appended ProGuard keep rules to $proguardPath"
    } else {
        Write-Host "ProGuard YtDlp keep rules already present."
    }
} else {
    Set-Content -Path $proguardPath -Value $overlayText -NoNewline
    Write-Host "Created $proguardPath with YtDlp keep rules"
}

Write-Host "Android YtDlp overlay patch complete."

# Remove stale Linux BtbN ffmpeg/ffprobe assets left from older tauri.android.conf.json
# resource mappings (they cannot run on Android/bionic and inflate the APK by ~260MB).
$staleBinaries = Join-Path $GenAndroid "app\src\main\assets\binaries"
if (Test-Path $staleBinaries) {
    Remove-Item -Recurse -Force $staleBinaries
    Write-Host "Removed stale assets/binaries (Linux ffmpeg sidecars)."
}