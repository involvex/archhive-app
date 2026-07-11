# Enables HTTP cleartext for Remote LAN mode on Android release builds.
param(
    [string]$GradlePath = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\build.gradle.kts")
)

if (-not (Test-Path $GradlePath)) {
    Write-Warning "Android project not found. Run: bun run tauri android init"
    exit 1
}

$content = Get-Content $GradlePath -Raw
if ($content -match 'manifestPlaceholders\["usesCleartextTraffic"\] = "true"') {
    Write-Host "Android LAN patch already applied."
    exit 0
}

$content = $content -replace 'manifestPlaceholders\["usesCleartextTraffic"\] = "false"', 'manifestPlaceholders["usesCleartextTraffic"] = "true"'
Set-Content -Path $GradlePath -Value $content -NoNewline
Write-Host "Patched $GradlePath for cleartext HTTP (Remote LAN)."
