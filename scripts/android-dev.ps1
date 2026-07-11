# Start Android emulator if needed, then run tauri android dev on a single device.
# Avoids the interactive "pick a device" hang when multiple AVDs exist but none is booted.

$ErrorActionPreference = "Stop"

function Get-AdbPath {
    if ($env:ANDROID_HOME) {
        $adb = Join-Path $env:ANDROID_HOME "platform-tools\adb.exe"
        if (Test-Path $adb) { return $adb }
    }
    if ($env:LOCALAPPDATA) {
        $adb = Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\adb.exe"
        if (Test-Path $adb) { return $adb }
    }
    $cmd = Get-Command adb -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    throw "adb not found. Install Android SDK platform-tools or set ANDROID_HOME."
}

function Get-EmulatorPath {
    if ($env:ANDROID_HOME) {
        $emu = Join-Path $env:ANDROID_HOME "emulator\emulator.exe"
        if (Test-Path $emu) { return $emu }
    }
    if ($env:LOCALAPPDATA) {
        $emu = Join-Path $env:LOCALAPPDATA "Android\Sdk\emulator\emulator.exe"
        if (Test-Path $emu) { return $emu }
    }
    $cmd = Get-Command emulator -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

function Get-BootedDevices([string]$Adb) {
    $lines = & $Adb devices 2>$null
    $booted = @()
    foreach ($line in $lines) {
        if ($line -match "^(?<id>\S+)\s+device$") {
            $booted += $Matches["id"]
        }
    }
    return $booted
}

$adb = Get-AdbPath
$devices = Get-BootedDevices $adb

if ($devices.Count -eq 0) {
    Write-Host "No booted Android device. Starting emulator..."
    $emulator = Get-EmulatorPath
    if (-not $emulator) {
        throw "No emulator found. Start an AVD in Android Studio or connect a USB device."
    }

    $avds = & $emulator -list-avds 2>$null
    if (-not $avds -or $avds.Count -eq 0) {
        throw "No AVDs configured. Create one in Android Studio Device Manager."
    }

    $avd = $avds[0]
    Write-Host "Launching AVD: $avd"
    Start-Process -FilePath $emulator -ArgumentList @("-avd", $avd) -WindowStyle Minimized | Out-Null

    Write-Host "Waiting for emulator to boot..."
    & $adb wait-for-device | Out-Null
    $deadline = (Get-Date).AddMinutes(3)
    do {
        $boot = & $adb shell getprop sys.boot_completed 2>$null
        if ($boot -match "1") { break }
        Start-Sleep -Seconds 2
    } while ((Get-Date) -lt $deadline)

    $devices = Get-BootedDevices $adb
}

if ($devices.Count -eq 0) {
    throw "Emulator failed to boot. Check Android Studio logs."
}

$device = $devices[0]
if ($devices.Count -gt 1) {
    Write-Host "Multiple devices; using first: $device"
    Write-Host "Pass a device id to override: bun run tauri android dev <device-id>"
}

Write-Host "Running tauri android dev on $device"
Set-Location (Join-Path $PSScriptRoot "..")
bun run tauri android dev $device @args
