# ArcHive AI stack setup - Stability Matrix + ComfyUI on I: drive
# Run: pwsh -NoProfile -ExecutionPolicy Bypass -File I:\setup-ai-env.ps1

$ErrorActionPreference = 'Stop'

function Write-Step([string]$Message) {
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Set-UserEnv([string]$Name, [string]$Value) {
    [Environment]::SetEnvironmentVariable($Name, $Value, 'User')
    Set-Item -Path "Env:$Name" -Value $Value
    Write-Host "  $Name = $Value"
}

function Move-Category {
    param(
        [string]$Source,
        [string]$Target,
        [string]$LogName
    )
    . (Join-Path $PSScriptRoot 'lib\supercopy-migrate.ps1')
    $parent = Split-Path $Target -Parent
    if (-not $parent) { $parent = 'I:\Models' }
    Invoke-SuperCopyDir -Source $Source -DestParent $parent -Move -LogName $LogName
}

Write-Step 'Setting user environment variables (keep C: clean)'
foreach ($dir in @('I:\huggingface', 'I:\pip-cache', 'I:\temp', 'I:\Models\diffusers')) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}
Set-UserEnv 'HF_HOME' 'I:\huggingface'
Set-UserEnv 'HUGGINGFACE_HUB_CACHE' 'I:\huggingface\hub'
Set-UserEnv 'TRANSFORMERS_CACHE' 'I:\huggingface\transformers'
Set-UserEnv 'OLLAMA_MODELS' 'I:\Models'
Set-UserEnv 'PIP_CACHE_DIR' 'I:\pip-cache'
Set-UserEnv 'TEMP' 'I:\temp'
Set-UserEnv 'TMP' 'I:\temp'

Write-Step 'Reorganizing existing I:\Models specials'
if (Test-Path 'I:\Models\sdxl-turbo') {
    Move-Category -Source 'I:\Models\sdxl-turbo' -Target 'I:\Models\diffusers\sdxl-turbo' -LogName 'sdxl-turbo'
}
if (Test-Path 'I:\Models\FLUX.1-schnell-GGUF') {
    New-Item -ItemType Directory -Path 'I:\Models\diffusion_models' -Force | Out-Null
    Move-Category -Source 'I:\Models\FLUX.1-schnell-GGUF' -Target 'I:\Models\diffusion_models\FLUX.1-schnell-GGUF' -LogName 'flux-gguf'
}

Write-Step 'Deduping Moody duplicates in diffusion_models (delete copies, keep moody\)'
$diffusion = 'I:\ComfyUI\ComfyUI\models\diffusion_models'
$moody = 'I:\ComfyUI\ComfyUI\models\moody'
if ((Test-Path $diffusion) -and (Test-Path $moody)) {
    Get-ChildItem $diffusion -File -ErrorAction SilentlyContinue | ForEach-Object {
        $dup = Join-Path $moody $_.Name
        if (Test-Path $dup) {
            Write-Host "  delete duplicate: $($_.Name)"
            Remove-Item $_.FullName -Force
        }
    }
}

Write-Step 'Moving ComfyUI model categories to I:\Models'
$categories = @(
    'checkpoints', 'diffusion_models', 'loras', 'moody', 'text_encoders', 'unet',
    'upscale_models', 'vae', 'vae_approx', 'sams', 'ultralytics', 'configs', 'images'
)
foreach ($cat in $categories) {
    Move-Category -Source "I:\ComfyUI\ComfyUI\models\$cat" -Target "I:\Models\$cat" -LogName $cat
}

Write-Step 'Moving Comfy Desktop Shared models'
$shared = 'I:\Comfy-Desktop\ComfyUI-Shared\models'
if (Test-Path $shared) {
    Get-ChildItem $shared -Directory -ErrorAction SilentlyContinue | ForEach-Object {
        Move-Category -Source $_.FullName -Target "I:\Models\$($_.Name)" -LogName "shared-$($_.Name)"
    }
}

Write-Step 'Moving SSD-1B safetensors to checkpoints'
$ssd = 'I:\huggingface\SSD-1B'
if (Test-Path $ssd) {
    New-Item -ItemType Directory -Path 'I:\Models\checkpoints' -Force | Out-Null
    Get-ChildItem $ssd -Filter '*.safetensors' -File -ErrorAction SilentlyContinue | ForEach-Object {
        $dest = Join-Path 'I:\Models\checkpoints' $_.Name
        if (-not (Test-Path $dest)) {
            Write-Host "  move $($_.Name)"
            Move-Item $_.FullName $dest
        }
    }
    if (Test-Path $ssd) {
        Move-Category -Source $ssd -Target 'I:\Models\diffusers\SSD-1B' -LogName 'ssd1b-diffusers'
    }
}

Write-Step 'Migrating C: HuggingFace cache to I:\huggingface (if present)'
$cHF = Join-Path $env:USERPROFILE '.cache\huggingface'
if (Test-Path -LiteralPath $cHF) {
    . (Join-Path $PSScriptRoot 'lib\supercopy-migrate.ps1')
    Get-ChildItem -LiteralPath $cHF -ErrorAction SilentlyContinue | ForEach-Object {
        Invoke-SuperCopyDir -Source $_.FullName -DestParent 'I:\huggingface' -Move -LogName "c-hf-$($_.Name)"
    }
}

Write-Step 'Creating ComfyUI extra_model_paths.yaml'
$extraYaml = @'
# Canonical shared model store for ComfyUI + Stability Matrix
shared_models:
  base_path: I:/Models/
  is_default: true
  checkpoints: checkpoints/
  diffusion_models: diffusion_models/
  loras: loras/
  vae: vae/
  vae_approx: vae_approx/
  text_encoders: text_encoders/
  clip: clip/
  clip_vision: clip_vision/
  controlnet: |-
    controlnet/
    t2i_adapter/
  upscale_models: upscale_models/
  embeddings: embeddings/
  hypernetworks: hypernetworks/
  unet: unet/
  gligen: gligen/
  photomaker: photomaker/
  style_models: style_models/
  sams: sams/
  ultralytics: ultralytics/
  configs: configs/
  diffusers: diffusers/
  audio_encoders: audio_encoders/
  model_patches: model_patches/
  latent_upscale_models: latent_upscale_models/
  moody: moody/
'@
Set-Content -Path 'I:\ComfyUI\ComfyUI\extra_model_paths.yaml' -Value $extraYaml -Encoding UTF8

Write-Step 'Creating junctions to I:\Models'
function Set-Junction([string]$Link, [string]$Target) {
    if (Test-Path $Link) {
        $item = Get-Item $Link -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            Write-Host "  already junction: $Link"
            return
        }
        if ((Get-ChildItem $Link -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
            Remove-Item $Link -Recurse -Force
        } else {
            Rename-Item $Link "$Link.old" -Force
        }
    }
    cmd /c mklink /J `"$Link`" `"$Target`"
    if ($LASTEXITCODE -ne 0) { throw "mklink failed: $Link -> $Target" }
}

Set-Junction 'I:\ComfyUI\ComfyUI\models' 'I:\Models'
Set-Junction 'I:\StabilityMatrix-win-x64\Data\Models' 'I:\Models'

Write-Step 'Updating Stability Matrix settings'
$smSettings = 'I:\StabilityMatrix-win-x64\Data\settings.json'
$sm = Get-Content $smSettings -Raw | ConvertFrom-Json
$sm.FirstLaunchSetupComplete = $true
$sm.HasSeenWelcomeNotification = $true
$sm | ConvertTo-Json -Depth 20 | Set-Content $smSettings -Encoding UTF8

Write-Step 'Updating Comfy Desktop settings (modelsDirs -> I:\Models only)'
$cdSettings = Join-Path $env:APPDATA 'Comfy Desktop\settings.json'
if (Test-Path $cdSettings) {
    $cd = Get-Content $cdSettings -Raw | ConvertFrom-Json
    $cd.modelsDirs = @('I:\Models')
    $cd | ConvertTo-Json -Depth 10 | Set-Content $cdSettings -Encoding UTF8
    $sharedYaml = Join-Path $env:APPDATA 'Comfy Desktop\shared_model_paths.yaml'
    if (Test-Path $sharedYaml) { Remove-Item $sharedYaml -Force }
}

Write-Step 'Done. Summary'
Write-Host "  Models root: I:\Models"
Write-Host "  HF cache:    I:\huggingface"
Write-Host "  Next: import I:\ComfyUI\ComfyUI in Stability Matrix Packages UI"
Write-Host "  Launch SM:   I:\StabilityMatrix-win-x64\StabilityMatrix.exe"
