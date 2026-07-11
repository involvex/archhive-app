# Patches generated Android project: cleartext HTTP + media storage permissions.
param(
    [string]$GradlePath = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\build.gradle.kts"),
    [string]$ManifestPath = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\src\main\AndroidManifest.xml")
)

if (-not (Test-Path $GradlePath)) {
    Write-Warning "Android project not found. Run: bun run tauri android init"
    exit 1
}

$content = Get-Content $GradlePath -Raw
if ($content -notmatch 'manifestPlaceholders\["usesCleartextTraffic"\] = "true"') {
    $content = $content -replace 'manifestPlaceholders\["usesCleartextTraffic"\] = "false"', 'manifestPlaceholders\["usesCleartextTraffic"\] = "true"'
    Set-Content -Path $GradlePath -Value $content -NoNewline
    Write-Host "Patched $GradlePath for cleartext HTTP (Remote LAN)."
} else {
    Write-Host "Android LAN cleartext patch already applied."
}

if (Test-Path $ManifestPath) {
    $manifest = Get-Content $ManifestPath -Raw
    if ($manifest -notmatch 'READ_MEDIA_VIDEO') {
        $manifest = $manifest -replace '(<uses-permission android:name="android.permission.INTERNET" />)', @'
$1
    <uses-permission android:name="android.permission.READ_MEDIA_VIDEO" />
    <uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" android:maxSdkVersion="32" />
'@
        Set-Content -Path $ManifestPath -Value $manifest -NoNewline
        Write-Host "Patched $ManifestPath with media storage permissions."
    } else {
        Write-Host "Android storage permissions already present."
    }
}
