# Legacy entry point - runs SuperCopy migration + junction wiring
$ErrorActionPreference = 'Stop'

& (Join-Path $PSScriptRoot 'migrate-models-supercopy.ps1') @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "`n==> Junctions + extra_model_paths.yaml" -ForegroundColor Cyan

@'
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
'@ | Set-Content 'I:\ComfyUI\ComfyUI\extra_model_paths.yaml' -Encoding UTF8

function Set-Junction([string]$Link, [string]$Target) {
    if (Test-Path -LiteralPath $Link) {
        $item = Get-Item -LiteralPath $Link -Force
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            Write-Host "  junction exists: $Link"
            return
        }
        $files = (Get-ChildItem -LiteralPath $Link -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
        if ($files -eq 0) { Remove-Item -LiteralPath $Link -Recurse -Force }
        else { Rename-Item -LiteralPath $Link -NewName "$(Split-Path $Link -Leaf).old" -Force }
    }
    cmd /c mklink /J "$Link" "$Target"
    if ($LASTEXITCODE -ne 0) { throw "mklink failed: $Link -> $Target" }
}

if (-not (Test-Path -LiteralPath 'I:\ComfyUI\ComfyUI\models')) {
    Set-Junction 'I:\ComfyUI\ComfyUI\models' 'I:\Models'
}
if (-not ((Get-Item -LiteralPath 'I:\StabilityMatrix-win-x64\Data\Models' -Force -ErrorAction SilentlyContinue).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
    Set-Junction 'I:\StabilityMatrix-win-x64\Data\Models' 'I:\Models'
}

Write-Host 'DONE continuation' -ForegroundColor Green
