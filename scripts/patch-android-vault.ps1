# Patches the generated Android project for the ArcHive media vault.
#
#   - MediaScanPlugin.kt            : MediaStore rescan bridge (gallery apps drop
#                                     stale rows for files moved into the vault)
#   - <uses-permission USE_BIOMETRIC> : fingerprint / face unlock for the vault
#
# Vault files live in app-private storage, which other apps cannot read and
# MediaStore never indexes, but a file that was moved out of a shared folder
# keeps its old MediaStore row until the path is rescanned. That is what the
# plugin is for.
#
# Run after `tauri android init` / android:regen (gen/android is gitignored).
# Idempotent: safe to re-run via apply-android-patches.ps1.
param(
    [string]$GenAndroid = (Join-Path $PSScriptRoot "..\src-tauri\gen\android"),
    [string]$Overlays = (Join-Path $PSScriptRoot "..\src-tauri\android-overlays")
)

$ErrorActionPreference = "Stop"

$gen = $GenAndroid
$overlayDir = $Overlays
$manifestPath = Join-Path $gen "app\src\main\AndroidManifest.xml"
$pluginSrc = Join-Path $overlayDir "MediaScanPlugin.kt"
$pluginDestDir = Join-Path $gen "app\src\main\java\com\archhive\app"
$pluginDest = Join-Path $pluginDestDir "MediaScanPlugin.kt"

if (-not (Test-Path (Join-Path $gen "app\build.gradle.kts"))) {
    Write-Warning "Android project not found at $gen. Run: bun run tauri android init"
    exit 1
}

if (-not (Test-Path $pluginSrc)) {
    throw "Missing overlay: $pluginSrc"
}

# 1. Kotlin plugin.
New-Item -ItemType Directory -Force -Path $pluginDestDir | Out-Null
Copy-Item -Path $pluginSrc -Destination $pluginDest -Force
Write-Host "Installed MediaScanPlugin.kt"

# 2. Manifest: biometric permission.
if (-not (Test-Path $manifestPath)) {
    throw "Missing manifest: $manifestPath"
}
$manifest = Get-Content $manifestPath -Raw
$permissionName = "android.permission.USE_BIOMETRIC"
if ($manifest -notmatch [regex]::Escape($permissionName)) {
    $permissionLine = "    <uses-permission android:name=""$permissionName"" />`r`n"
    $anchor = "    <application"
    $idx = $manifest.IndexOf($anchor)
    if ($idx -lt 0) {
        throw "Could not find <application> anchor in $manifestPath"
    }
    $manifest = $manifest.Substring(0, $idx) + $permissionLine + $manifest.Substring($idx)
    Set-Content -Path $manifestPath -Value $manifest -NoNewline
    Write-Host "Patched AndroidManifest with USE_BIOMETRIC."
} else {
    Write-Host "USE_BIOMETRIC already present."
}

Write-Host "Android vault patch complete."
