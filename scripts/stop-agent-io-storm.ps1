# Stop hung Cursor-agent I: drive I/O storm. Run once from C: in an elevated or normal pwsh.
# Does NOT delete data. Safe to re-run.

$ErrorActionPreference = 'SilentlyContinue'

Write-Host "Stopping robocopy..."
Get-Process robocopy -ErrorAction SilentlyContinue | Stop-Process -Force

$patterns = @(
    'finish-migrate',
    'migrate-models-supercopy',
    'setup-ai-env',
    'repair_directml',
    'run_comfy_headless'
)

Write-Host "Stopping pwsh children matching migration/repair scripts..."
Get-CimInstance Win32_Process -Filter "Name='pwsh.exe' OR Name='powershell.exe'" |
    Where-Object {
        $cmd = $_.CommandLine
        if (-not $cmd) { return $false }
        foreach ($p in $patterns) {
            if ($cmd -like "*$p*") { return $true }
        }
        $false
    } |
    ForEach-Object {
        Write-Host "  PID $($_.ProcessId): $($_.CommandLine.Substring(0, [Math]::Min(120, $_.CommandLine.Length)))"
        Stop-Process -Id $_.ProcessId -Force
    }

Write-Host "Done. Let I: idle 2-3 minutes before starting ONE migration job."
