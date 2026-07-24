# Register existing ComfyUI portable install in Stability Matrix settings.
# Run after setup-ai-env.ps1: pwsh -NoProfile -File D:\repos\archhive-app\scripts\setup-sm-comfyui.ps1

$ErrorActionPreference = 'Stop'

$smSettingsPath = 'I:\StabilityMatrix-win-x64\Data\settings.json'
$packagePath = 'I:\ComfyUI\ComfyUI'
$packageId = [guid]::NewGuid().ToString()

$launchArgs = @(
    '--windows-standalone-build',
    '--directml',
    '--lowvram',
    '--disable-dynamic-vram',
    '--port', '8188',
    '--listen', '127.0.0.1',
    '--enable-manager'
) -join ' '

$package = [ordered]@{
    Id                  = $packageId
    DisplayName         = 'ComfyUI (Portable I:)'
    PackageName         = 'ComfyUI'
    Version             = 'Imported'
    Path                = $packagePath
    PythonVersion       = '3.12.10'
    InstalledReleaseVersion = ''
    LaunchArgs          = $launchArgs
    LastUpdateCheck     = (Get-Date).ToUniversalTime().ToString('o')
    UpdateAvailable     = $false
    DontUpdate          = $false
    PreferredTorchIndex = $null
    PreferredGpu        = $null
}

$settings = Get-Content $smSettingsPath -Raw | ConvertFrom-Json
$settings.FirstLaunchSetupComplete = $true
$settings.HasSeenWelcomeNotification = $true
$settings.PreferredUpdateChannel = 'Stable'

$existing = @()
if ($null -ne $settings.InstalledPackages) {
    $existing = @($settings.InstalledPackages)
}

$already = $existing | Where-Object { $_.Path -eq $packagePath }
if ($already) {
    Write-Host "ComfyUI already registered at $packagePath"
} else {
    $settings.InstalledPackages = @($existing + $package)
    Write-Host "Registered ComfyUI package: $packagePath"
}

$settings | ConvertTo-Json -Depth 20 | Set-Content $smSettingsPath -Encoding UTF8
Write-Host "Updated $smSettingsPath"
