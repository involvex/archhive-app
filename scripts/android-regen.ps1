# Regenerate Android project (fixes stale package paths after identifier changes).
# Also applies LAN cleartext patch and Kotlin cross-drive workarounds for I:\ vs C:\ projects.

$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$genAndroid = Join-Path $root "src-tauri\gen\android"

if (Test-Path $genAndroid) {
    Write-Host "Removing stale src-tauri/gen/android ..."
    Remove-Item -Recurse -Force $genAndroid
}

Set-Location $root
Write-Host "Running tauri android init ..."
bun run tauri android init

$gradleProps = Join-Path $genAndroid "gradle.properties"
if (-not (Test-Path $gradleProps)) {
    throw "gradle.properties not found after android init."
}

$kotlinFixes = @(
    "kotlin.incremental=false",
    "kotlin.compiler.execution.strategy=in-process"
)

$content = Get-Content $gradleProps -Raw
foreach ($line in $kotlinFixes) {
    $key = ($line -split "=")[0]
    if ($content -notmatch "(?m)^$key=") {
        $content = $content.TrimEnd() + "`n$line`n"
        Write-Host "Added $line to gradle.properties"
    }
}
Set-Content -Path $gradleProps -Value $content -NoNewline

$gradlew = Join-Path $genAndroid "gradlew.bat"
if (Test-Path $gradlew) {
    Write-Host "Stopping Gradle/Kotlin daemons ..."
    & $gradlew --stop 2>$null | Out-Null
}

& (Join-Path $PSScriptRoot "patch-android-lan.ps1")

$iconSource = Join-Path $root "assets\branding\icon-source.png"
$iconSquare = Join-Path $root "assets\branding\icon-square.png"
$iconInput = if (Test-Path $iconSource) { $iconSource } elseif (Test-Path $iconSquare) { $iconSquare } else { $null }
if ($iconInput) {
    Write-Host "Applying launcher icons from $iconInput ..."
    bun run tauri icon $iconInput
} else {
    Write-Host "No assets/branding/icon-source.png — skip tauri icon (Android keeps default launcher)."
}

Write-Host "Android project regenerated."
Write-Host "Note: tauri.settings.gradle is created on the first `tauri android dev` run."
Write-Host "Run: bun run android:dev"
