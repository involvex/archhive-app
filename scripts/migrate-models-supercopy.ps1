# Migrate ComfyUI model categories to I:\Models using SuperCopy (fast on slow HDD)
# Usage:
#   pwsh -NoProfile -File D:\repos\archhive-app\scripts\migrate-models-supercopy.ps1
#   pwsh -NoProfile -File D:\repos\archhive-app\scripts\migrate-models-supercopy.ps1 -Category moody
#   pwsh -NoProfile -File D:\repos\archhive-app\scripts\migrate-models-supercopy.ps1 -Workers 6

param(
    [string]$Category = '',
    [int]$Workers = 4,
    [switch]$Verify,
    [switch]$SkipSpecials
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\supercopy-migrate.ps1')

function Write-Step([string]$Message) {
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

$allCategories = @(
    'configs', 'vae_approx', 'vae', 'images', 'ultralytics', 'sams',
    'upscale_models', 'loras', 'checkpoints', 'text_encoders', 'unet',
    'diffusion_models', 'moody'
)

if ($Category) {
    $allCategories = @($Category)
}

if (-not $SkipSpecials) {
    Write-Step 'Dedupe Moody duplicates in diffusion_models (delete only)'
    $diffusion = 'I:\ComfyUI\ComfyUI\models\diffusion_models'
    $moody = 'I:\ComfyUI\ComfyUI\models\moody'
    if ((Test-Path -LiteralPath $diffusion) -and (Test-Path -LiteralPath $moody)) {
        Get-ChildItem -LiteralPath $diffusion -File -ErrorAction SilentlyContinue | ForEach-Object {
            if (Test-Path -LiteralPath (Join-Path $moody $_.Name)) {
                Write-Host "  delete duplicate: $($_.Name)"
                Remove-Item -LiteralPath $_.FullName -Force
            }
        }
    }

    Write-Step 'Reorganize I:\Models specials'
    $sdxlDest = 'I:\Models\diffusers\sdxl-turbo'
    if (Test-Path -LiteralPath 'I:\Models\sdxl-turbo') {
        if ((Test-Path -LiteralPath $sdxlDest) -and (Get-ChildItem -LiteralPath $sdxlDest -Recurse -File -ErrorAction SilentlyContinue).Count -gt 0) {
            Write-Host "  skip sdxl-turbo (already at $sdxlDest); removing stale source"
            Remove-Item -LiteralPath 'I:\Models\sdxl-turbo' -Recurse -Force -ErrorAction SilentlyContinue
        } else {
            Invoke-SuperCopyDir -Source 'I:\Models\sdxl-turbo' -DestParent 'I:\Models\diffusers' -Move -Workers $Workers -Verify:$Verify -LogName 'sdxl-turbo'
        }
    }
    if (Test-Path -LiteralPath 'I:\Models\FLUX.1-schnell-GGUF') {
        $fluxDest = 'I:\Models\diffusion_models\FLUX.1-schnell-GGUF'
        if ((Test-Path -LiteralPath $fluxDest) -and (Get-ChildItem -LiteralPath $fluxDest -Recurse -File -ErrorAction SilentlyContinue).Count -gt 0) {
            Write-Host "  skip FLUX GGUF (already at $fluxDest); removing stale source"
            Remove-Item -LiteralPath 'I:\Models\FLUX.1-schnell-GGUF' -Recurse -Force -ErrorAction SilentlyContinue
        } else {
            Invoke-SuperCopyDir -Source 'I:\Models\FLUX.1-schnell-GGUF' -DestParent 'I:\Models\diffusion_models' -Move -Workers $Workers -LogName 'flux-gguf'
        }
    }
}

Write-Step 'ComfyUI model categories -> I:\Models'
foreach ($cat in $allCategories) {
    Invoke-SuperCopyDir `
        -Source "I:\ComfyUI\ComfyUI\models\$cat" `
        -DestParent 'I:\Models' `
        -Move `
        -Workers $Workers `
        -LogName $cat `
        -Verify:$Verify
}

if (-not $Category) {
    Write-Step 'Comfy Desktop Shared models -> I:\Models'
    $shared = 'I:\Comfy-Desktop\ComfyUI-Shared\models'
    if (Test-Path -LiteralPath $shared) {
        Get-ChildItem -LiteralPath $shared -Directory -ErrorAction SilentlyContinue | ForEach-Object {
            Invoke-SuperCopyDir -Source $_.FullName -DestParent 'I:\Models' -Move -Workers $Workers -LogName "shared-$($_.Name)"
        }
    }

    Write-Step 'SSD-1B safetensors -> checkpoints'
    $ssd = 'I:\huggingface\SSD-1B'
    if (Test-Path -LiteralPath $ssd) {
        New-Item -ItemType Directory -Path 'I:\Models\checkpoints' -Force | Out-Null
        Get-ChildItem -LiteralPath $ssd -Filter '*.safetensors' -File -ErrorAction SilentlyContinue | ForEach-Object {
            Invoke-SuperCopyFile -SourceFile $_.FullName -DestDir 'I:\Models\checkpoints' -Move
        }
        if ((Get-ChildItem -LiteralPath $ssd -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0) {
            Invoke-SuperCopyDir -Source $ssd -DestParent 'I:\Models\diffusers' -Move -Workers $Workers -LogName 'ssd1b-diffusers'
        }
    }

    Write-Step 'C: HuggingFace cache -> I:\huggingface'
    $cHF = Join-Path $env:USERPROFILE '.cache\huggingface'
    if (Test-Path -LiteralPath $cHF) {
        Get-ChildItem -LiteralPath $cHF -ErrorAction SilentlyContinue | ForEach-Object {
            Invoke-SuperCopyDir -Source $_.FullName -DestParent 'I:\huggingface' -Move -Workers $Workers -LogName "c-hf-$($_.Name)"
        }
    }
}

Write-Host "`nDone. Logs: I:\Models\_migrate-*.log" -ForegroundColor Green
