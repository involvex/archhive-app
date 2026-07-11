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

function Get-PcLanIp {
    $ip = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object {
            $_.IPAddress -match '^192\.168\.' -and
            $_.PrefixOrigin -ne 'WellKnown' -and
            $_.IPAddress -ne '127.0.0.1'
        } |
        Select-Object -First 1 -ExpandProperty IPAddress
    if ($ip) { return $ip }
    return $null
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

$pcIp = Get-PcLanIp
$apiUrl = if ($pcIp) { "http://${pcIp}:8787" } else { "http://<pc-lan-ip>:8787" }

Write-Host "Starting desktop LAN host (port 8787, open mode)..."
& (Join-Path $PSScriptRoot "start-lan-host.ps1") -Port 8787

Write-Host "Running tauri android dev on $device"
if ($device -notmatch "^emulator-") {
    Write-Host ""
    Write-Host "Physical device on Wi-Fi:"
    Write-Host "  1. Phone and PC must be on the same network (you can ping the phone)."
    Write-Host "  2. In the app: Settings -> Engine -> tap a host under LAN discovery"
    Write-Host "  3. Expected desktop API: $apiUrl (port 8787 only, not 1420)"
    Write-Host "  4. Leave token empty when desktop was started via android:dev"
    Write-Host ""
} else {
    Write-Host "Emulator: use discovered host 10.0.2.2:8787 in Settings -> Engine"
}
Set-Location (Join-Path $PSScriptRoot "..")
bun run tauri android dev $device @args
