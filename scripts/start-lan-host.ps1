# Start desktop ArcHive with LAN auto-enabled (ARCHIVE_AUTO_LAN=1) if port 8787 is down or locked.
param(
    [int]$Port = 8787,
    [int]$WaitSeconds = 90
)

$ErrorActionPreference = "Stop"
$root = Join-Path $PSScriptRoot ".."

function Get-LanHealth([string]$Url) {
    try {
        $r = Invoke-WebRequest -Uri $Url -TimeoutSec 3 -UseBasicParsing
        if ($r.StatusCode -ne 200) { return $null }
        return ($r.Content | ConvertFrom-Json)
    } catch {
        return $null
    }
}

function Stop-PortListener([int]$Port) {
    try {
        $conns = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
        foreach ($c in $conns) {
            if ($c.OwningProcess -gt 0) {
                Write-Host "Stopping process $($c.OwningProcess) on port $Port"
                Stop-Process -Id $c.OwningProcess -Force -ErrorAction SilentlyContinue
            }
        }
        Start-Sleep -Seconds 1
    } catch {
        Write-Warning "Could not stop listener on port ${Port}: $_"
    }
}

$healthUrl = "http://127.0.0.1:$Port/api/health"
$health = Get-LanHealth $healthUrl
if ($health -and $health.auth_required -eq $false) {
    Write-Host "LAN host already running (open mode) on port $Port"
    Write-Host "  auth_required: false"
    $lanIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -like '192.168.*' -and $_.PrefixOrigin -ne 'WellKnown' } |
        Select-Object -First 1).IPAddress
    if ($lanIp) {
        Write-Host "  PC LAN URL: http://${lanIp}:$Port"
    }
    exit 0
}

if ($health -and $health.auth_required -eq $true) {
    Write-Host "LAN host on port $Port requires a token - restarting in open dev mode..."
    Write-Host "  auth_required: true (token-locked - mobile will get 401)"
    Stop-PortListener $Port
}

Write-Host "Starting desktop ArcHive (LAN auto-start on port $Port)..."
$env:ARCHIVE_AUTO_LAN = "1"
Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "Set-Location '$root'; `$env:ARCHIVE_AUTO_LAN='1'; bun run tauri dev"
) | Out-Null

$deadline = (Get-Date).AddSeconds($WaitSeconds)
do {
    $health = Get-LanHealth $healthUrl
    if ($health -and $health.auth_required -eq $false) {
        Write-Host "LAN host ready at $healthUrl (no token required)"
        Write-Host "  auth_required: false"
        $lanIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
            Where-Object { $_.IPAddress -like '192.168.*' -and $_.PrefixOrigin -ne 'WellKnown' } |
            Select-Object -First 1).IPAddress
        if ($lanIp) {
            Write-Host "  PC LAN URL: http://${lanIp}:$Port"
        } else {
            Write-Warning "Could not detect 192.168.* LAN IP - run ipconfig and use http://<pc-ip>:$Port"
        }
        exit 0
    }
    Start-Sleep -Seconds 2
} while ((Get-Date) -lt $deadline)

Write-Warning "LAN host did not respond in open mode within ${WaitSeconds}s. Start desktop manually: bun run tauri dev"
exit 1
