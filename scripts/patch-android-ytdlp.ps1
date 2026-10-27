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
$cookieBrowserSrc = Join-Path $Overlays "CookieBrowserPlugin.kt"
if (Test-Path $cookieBrowserSrc) {
    Copy-Item -Path $cookieBrowserSrc -Destination (Join-Path $pluginDestDir "CookieBrowserPlugin.kt") -Force
    Write-Host "Installed CookieBrowserPlugin.kt"
}
# FileProvider roots: shared storage + app-private dirs ("Open with default app").
$filePathsSrc = Join-Path $Overlays "file_paths.xml"
$filePathsDest = Join-Path $GenAndroid "app\src\main\res\xml\file_paths.xml"
if (Test-Path $filePathsSrc) {
    New-Item -ItemType Directory -Force -Path (Split-Path $filePathsDest) | Out-Null
    Copy-Item -Path $filePathsSrc -Destination $filePathsDest -Force
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

# R8 minification keeps breaking release builds: v0.6.0 boot-crashed (commons-compress
# reflection), v0.6.1/v0.6.2 broke every plugin invoke (Jackson Args creators stripped),
# while debug builds with identical code worked. Release parity with debug > APK size,
# so disable minification for the release buildType.
if ($gradle -notmatch [regex]::Escape('isMinifyEnabled = false // r8-off marker')) {
    if ($gradle -match '(?s)getByName\("release"\)\s*\{[^}]*?isMinifyEnabled\s*=\s*true') {
        # Replace the release-block assignment (the debug block keeps its own).
        $gradle = [regex]::Replace(
            $gradle,
            '(?s)(getByName\("release"\)\s*\{[^}]*?)isMinifyEnabled\s*=\s*true',
            "`$1isMinifyEnabled = false // r8-off marker"
        )
        $changed = $true
        Write-Host "Disabled R8 minification for release buildType (debug parity)."
    } else {
        Write-Warning "Release buildType has no isMinifyEnabled = true to disable in $gradlePath"
    }
}

# Chaquopy opens native libs (libpython.zip.so) by file path at runtime, so
# they must be extracted at install (legacy packaging), matching
# android:extractNativeLibs="true" in the manifest. Without this, release
# builds keep .so uncompressed in-APK and YoutubeDL.init fails with ENOENT.
if ($gradle -notmatch 'useLegacyPackaging') {
    $packagingBlock = @'
    packaging {
        // youtubedl-android/Chaquopy need on-disk native libs (libpython.zip.so)
        jniLibs {
            useLegacyPackaging = true
        }
    }
'@
    if ($gradle -match '(?m)^(    buildTypes \{)') {
        $gradle = $gradle -replace '(?m)^(    buildTypes \{)', "$packagingBlock`r`n`$1"
        $changed = $true
        Write-Host "Enabled legacy native-lib packaging (extract .so at install)"
    } else {
        Write-Warning "Could not find buildTypes block in $gradlePath"
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

# Projects patched before these rules existed: add the library keeps that
# stop R8 from breaking YoutubeDL.init / FFmpeg.init in release builds.
$proguardNow = Get-Content $proguardPath -Raw
if ($proguardNow -notmatch 'org\.apache\.commons\.compress') {
    $libKeeps = @"
-keep class org.apache.commons.compress.** { *; }
-keep class org.apache.commons.io.** { *; }
-keep class com.fasterxml.jackson.** { *; }
-keepattributes Signature,*Annotation*,InnerClasses,EnclosingMethod
-dontwarn org.apache.commons.compress.**
-dontwarn org.apache.commons.io.**
-dontwarn com.fasterxml.jackson.**
"@
    $proguardNow = $proguardNow.TrimEnd() + "`r`n" + $libKeeps
    Set-Content -Path $proguardPath -Value $proguardNow -NoNewline
    Write-Host "Added youtubedl-android library ProGuard keeps."
}

# Projects patched before the cookie login browser existed: add its keeps.
$proguardNow = Get-Content $proguardPath -Raw
if ($proguardNow -notmatch 'CookieBrowserPlugin') {
    $cookieKeeps = "-keep class com.archhive.app.CookieBrowserPlugin { *; }`r`n-keep class com.archhive.app.CookieBrowserPlugin`$OpenArgs { *; }`r`n"
    Set-Content -Path $proguardPath -Value ($proguardNow.TrimEnd() + "`r`n" + $cookieKeeps) -NoNewline
    Write-Host "Added CookieBrowserPlugin ProGuard keeps."
}

# Projects patched before the plugin Args keep existed: R8 stripped the
# nested *Args classes, breaking every yt-dlp invoke in release builds.
$proguardNow = Get-Content $proguardPath -Raw
if ($proguardNow -notmatch [regex]::Escape('com.archhive.app.**Args')) {
    $argsKeep = "-keep class com.archhive.app.**Args { *; }`r`n"
    Set-Content -Path $proguardPath -Value ($proguardNow.TrimEnd() + "`r`n" + $argsKeep) -NoNewline
    Write-Host "Added plugin Args ProGuard keeps."
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
