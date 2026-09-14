# Start Android emulator if needed, then run tauri android dev on a single device.
# Avoids the interactive "pick a device" hang when multiple AVDs exist but none is booted.
#
# Usage:
#   bun run android:dev                  # Android only (recommended — no cargo lock fight)
#   bun run android:dev -- -WithLanHost  # Also start desktop LAN (separate cargo target)
#   bun run tauri:android:dev            # Same helper (alias)
#
# "Website not available" in the Android WebView usually means the device cannot reach
# Vite on the PC. This script sets --host / TAURI_DEV_HOST to your LAN IP.

param(
    [switch]$WithLanHost,
    [string]$Device = "",
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Passthrough = @()
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

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
    $candidates = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object {
            $_.IPAddress -ne '127.0.0.1' -and
            $_.PrefixOrigin -ne 'WellKnown' -and
            (
                $_.IPAddress -match '^192\.168\.' -or
                $_.IPAddress -match '^10\.' -or
                $_.IPAddress -match '^172\.(1[6-9]|2[0-9]|3[0-1])\.'
            )
        } |
        Sort-Object {
            # Prefer 192.168.* (home LAN) over VPN/WSL-ish ranges
            if ($_.IPAddress -match '^192\.168\.') { 0 }
            elseif ($_.IPAddress -match '^10\.') { 1 }
            else { 2 }
        }
    if ($candidates) {
        return ($candidates | Select-Object -First 1 -ExpandProperty IPAddress)
    }
    return $null
}

function Test-HttpUp([string]$Url) {
    try {
        $r = Invoke-WebRequest -Uri $Url -TimeoutSec 2 -UseBasicParsing
        return ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500)
    } catch {
        return $false
    }
}

function Set-AndroidCargoTargetDir {
    # Desktop `tauri dev` and `tauri android dev` share one global target-dir by
    # default (often I:/dev/cargo-target). Parallel cargo invocations then fail
    # with "Waiting for file lock on package cache / build directory".
    if ($env:CARGO_TARGET_DIR_ANDROID) {
        $env:CARGO_TARGET_DIR = $env:CARGO_TARGET_DIR_ANDROID
        Write-Host "CARGO_TARGET_DIR (android): $($env:CARGO_TARGET_DIR)"
        return
    }
    $base = $env:CARGO_TARGET_DIR
    if (-not $base) {
        $base = Join-Path $root "src-tauri\target"
    }
    $parent = Split-Path $base -Parent
    $leaf = Split-Path $base -Leaf
    $androidTarget = Join-Path $parent "$leaf-android"
    $env:CARGO_TARGET_DIR = $androidTarget
    Write-Host "CARGO_TARGET_DIR (android): $androidTarget"
    Write-Host "  (desktop keeps '$base' — avoids cargo file locks)"
}

Set-Location $root
Set-AndroidCargoTargetDir

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

if ($Device) {
    if ($devices -notcontains $Device) {
        throw "Device '$Device' not in adb devices: $($devices -join ', ')"
    }
    $device = $Device
} else {
    $device = $devices[0]
    if ($devices.Count -gt 1) {
        Write-Host "Multiple devices; using first: $device"
        Write-Host "Override: bun run android:dev -- -Device <id>"
    }
}

$pcIp = Get-PcLanIp
if (-not $pcIp) {
    throw "Could not detect a LAN IPv4 address. Connect Wi-Fi/Ethernet, or set TAURI_DEV_HOST manually."
}

# Mobile WebView cannot use localhost (that is the phone/emulator itself).
$env:TAURI_DEV_HOST = $pcIp
Write-Host "TAURI_DEV_HOST=$pcIp (Vite must be reachable at http://${pcIp}:1420)"

$apiUrl = "http://${pcIp}:8787"
$viteUp = Test-HttpUp "http://127.0.0.1:1420"

if ($WithLanHost) {
    Write-Host "Starting desktop LAN host (port 8787, open mode)..."
    & (Join-Path $PSScriptRoot "start-lan-host.ps1") -Port 8787
    $viteUp = Test-HttpUp "http://127.0.0.1:1420"
} else {
    $lanHealth = $null
    try {
        $lanHealth = (Invoke-WebRequest -Uri "http://127.0.0.1:8787/api/health" -TimeoutSec 2 -UseBasicParsing).Content | ConvertFrom-Json
    } catch {}
    if ($lanHealth) {
        Write-Host "Desktop LAN already up on :8787 (auth_required=$($lanHealth.auth_required))"
    } else {
        Write-Host "Skipping desktop tauri (no cargo lock fight). Standalone Local engine works offline."
        Write-Host "  Remote LAN later: start desktop in another terminal, or re-run with -WithLanHost"
    }
}

Write-Host "Running tauri android dev on $device"
if ($device -notmatch "^emulator-") {
    Write-Host ""
    Write-Host "Physical device:"
    Write-Host "  - Dev UI: http://${pcIp}:1420 (must load in phone browser for a quick check)"
    Write-Host "  - Remote LAN API: $apiUrl (Settings -> Engine), not port 1420"
    Write-Host ""
} else {
    Write-Host "Emulator Remote LAN tip: Settings -> Engine -> http://10.0.2.2:8787"
    Write-Host "  (dev UI still uses PC LAN IP $pcIp:1420 via TAURI_DEV_HOST)"
}

$tauriArgs = @("run", "tauri", "android", "dev", $device, "--host", $pcIp)
if ($viteUp) {
    Write-Host "Reusing Vite already on :1420 (skipping beforeDevCommand — avoids port/cargo fights)"
    $tauriArgs += @("--config", '{"build":{"beforeDevCommand":""}}')
}
if ($Passthrough.Count -gt 0) {
    $tauriArgs += $Passthrough
}

bun @tauriArgs
