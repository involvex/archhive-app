# Patches generated Android project with the ArcHive share target.
# ArcHive appears in the Android share sheet for text/plain (single + multiple),
# MainActivity forwards ACTION_SEND intents to ShareIntentPlugin, and the
# webview pulls the pending text via get_pending_share.
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
$mainActivityPath = Join-Path $gen "app\src\main\java\com\archhive\app\MainActivity.kt"
$pluginSrc = Join-Path $overlayDir "ShareIntentPlugin.kt"
$pluginDestDir = Join-Path $gen "app\src\main\java\com\archhive\app"
$pluginDest = Join-Path $pluginDestDir "ShareIntentPlugin.kt"

if (-not (Test-Path (Join-Path $gen "app\build.gradle.kts"))) {
    Write-Warning "Android project not found at $gen. Run: bun run tauri android init"
    exit 1
}

if (-not (Test-Path $pluginSrc)) {
    throw "Missing overlay: $pluginSrc"
}

New-Item -ItemType Directory -Force -Path $pluginDestDir | Out-Null
Copy-Item -Path $pluginSrc -Destination $pluginDest -Force
Write-Host "Installed ShareIntentPlugin.kt"

# 1. Manifest: SEND + SEND_MULTIPLE intent filters inside .MainActivity.
if (-not (Test-Path $manifestPath)) {
    throw "Missing manifest: $manifestPath"
}
$manifest = Get-Content $manifestPath -Raw
if ($manifest -notmatch 'android\.intent\.action\.SEND') {
    $shareFilters = @'
            <intent-filter android:label="Add to ArcHive">
                <action android:name="android.intent.action.SEND" />
                <category android:name="android.intent.category.DEFAULT" />
                <data android:mimeType="text/plain" />
            </intent-filter>
            <intent-filter android:label="Add to ArcHive">
                <action android:name="android.intent.action.SEND_MULTIPLE" />
                <category android:name="android.intent.category.DEFAULT" />
                <data android:mimeType="text/plain" />
            </intent-filter>
'@
    # Insert after the LAUNCHER intent-filter closing tag (first occurrence).
    $anchor = '</intent-filter>'
    $idx = $manifest.IndexOf($anchor)
    if ($idx -lt 0) {
        throw "Could not find intent-filter anchor in $manifestPath"
    }
    $manifest = $manifest.Substring(0, $idx + $anchor.Length) + "`r`n" + $shareFilters + $manifest.Substring($idx + $anchor.Length)
    Set-Content -Path $manifestPath -Value $manifest -NoNewline
    Write-Host "Patched AndroidManifest with share-target intent filters."
} else {
    Write-Host "Android share-target intent filters already present."
}

# 2. MainActivity: forward intents to ShareIntentPlugin (cold start + warm share).
if (-not (Test-Path $mainActivityPath)) {
    throw "Missing MainActivity: $mainActivityPath"
}
$activity = Get-Content $mainActivityPath -Raw
$activityChanged = $false

if ($activity -notmatch 'import android\.content\.Intent') {
    $activity = $activity -replace '(package com\.archhive\.app\s*)', "`$1`r`nimport android.content.Intent`r`n"
    $activityChanged = $true
}

if ($activity -notmatch 'ShareIntentPlugin') {
    $oldCreate = '    super.onCreate(savedInstanceState)'
    $newCreate = '    super.onCreate(savedInstanceState)
    ShareIntentPlugin.setPendingFromIntent(intent)'
    if ($activity -match [regex]::Escape($oldCreate)) {
        $activity = $activity -replace [regex]::Escape($oldCreate), $newCreate
        $activityChanged = $true
    }
    $lastBrace = $activity.LastIndexOf('}')
    if ($lastBrace -gt 0) {
        $newIntentBlock = @'

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    setIntent(intent)
    ShareIntentPlugin.setPendingFromIntent(intent)
  }
'@
        $activity = $activity.Substring(0, $lastBrace) + $newIntentBlock + "`r`n}`r`n"
        $activityChanged = $true
    }
}

if ($activityChanged) {
    Set-Content -Path $mainActivityPath -Value $activity -NoNewline
    Write-Host "Patched MainActivity.kt with share-intent forwarding."
} else {
    Write-Host "MainActivity.kt share-intent forwarding already present."
}

Write-Host "Android share-target patch complete."
