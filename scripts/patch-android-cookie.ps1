# Patches the generated Android project with the ArcHive cookie-extractor flow and
# the Termux RUN_COMMAND bridge.
#
#   - CookieHarvestPlugin.kt  : in-app WebView login window + cookie harvest bridge
#   - TermuxBridgePlugin.kt   : explicit RUN_COMMAND intents to the custom Termux fork
#                               (com.involvex.termux_app, Java namespace com.invapp)
#   - <activity .CookieLoginActivity> registered in the manifest
#   - <uses-permission com.involvex.termux_app.permission.RUN_COMMAND> so ArcHive may
#     hand work to Termux
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
$javaDir = Join-Path $gen "app\src\main\java\com\archhive\app"

if (-not (Test-Path (Join-Path $gen "app\build.gradle.kts"))) {
    Write-Warning "Android project not found at $gen. Run: bun run tauri android init"
    exit 1
}

New-Item -ItemType Directory -Force -Path $javaDir | Out-Null

foreach ($overlay in @("CookieHarvestPlugin.kt", "TermuxBridgePlugin.kt")) {
    $src = Join-Path $overlayDir $overlay
    if (-not (Test-Path $src)) {
        throw "Missing overlay: $src"
    }
    Copy-Item -Path $src -Destination (Join-Path $javaDir $overlay) -Force
    Write-Host "Installed $overlay"
}

if (-not (Test-Path $manifestPath)) {
    throw "Missing manifest: $manifestPath"
}

$manifest = Get-Content $manifestPath -Raw
$changed = $false

# 1. RUN_COMMAND permission (request ArcHive may run commands in the Termux fork).
$permissionName = "com.involvex.termux_app.permission.RUN_COMMAND"
if ($manifest -notmatch [regex]::Escape($permissionName)) {
    $permissionLine = "    <uses-permission android:name=""$permissionName"" />`r`n"
    $anchor = "    <application"
    $idx = $manifest.IndexOf($anchor)
    if ($idx -lt 0) {
        throw "Could not find <application> anchor in $manifestPath"
    }
    $manifest = $manifest.Substring(0, $idx) + $permissionLine + $manifest.Substring($idx)
    $changed = $true
    Write-Host "Patched AndroidManifest with Termux RUN_COMMAND permission."
} else {
    Write-Host "Termux RUN_COMMAND permission already present."
}

# 2. Register the cookie-login window as a non-exported activity.
if ($manifest -notmatch 'CookieLoginActivity') {
    $activity = @'
        <activity
            android:name=".CookieLoginActivity"
            android:exported="false"
            android:configChanges="orientation|keyboardHidden|screenSize|smallestScreenSize|screenLayout|uiMode"
            android:theme="@style/Theme.archhive_app" />
'@
    $anchor = '</application>'
    $idx = $manifest.IndexOf($anchor)
    if ($idx -lt 0) {
        throw "Could not find </application> anchor in $manifestPath"
    }
    $manifest = $manifest.Substring(0, $idx) + $activity + "`r`n" + $manifest.Substring($idx)
    $changed = $true
    Write-Host "Patched AndroidManifest with CookieLoginActivity."
} else {
    Write-Host "CookieLoginActivity already registered."
}

if ($changed) {
    Set-Content -Path $manifestPath -Value $manifest -NoNewline
}

Write-Host "Android cookie-extractor + Termux patch complete."
