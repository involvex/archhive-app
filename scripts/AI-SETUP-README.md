# ArcHive AI - Daily Guide (AMD RX 6600)

## One app to open

Double-click: `I:\StabilityMatrix-win-x64\Start AI.lnk`

## Simple images (no nodes)

1. Stability Matrix -> **Inference**
2. Pick a checkpoint
3. Type prompt -> **Generate**

## Comfy workflows (advanced)

1. Stability Matrix -> **Packages** -> **ComfyUI (Portable I:)** -> **Launch**
2. Load workflow -> Queue
3. Close when done (do not leave running in background)

## Model location

All models: `I:\Models\`

Categories (desktop): `checkpoints`, `loras`, `moody`, `vae`

Format cheat sheet: `I:\Models\MODEL-GUIDE.md`

**Ignore on RX 6600:** `diffusers/` (HF duplicates), `text_encoders/qwen_*`, mobile `sd_*` folders, QNN toolchain — not Comfy weights.

## If ComfyUI will not start

Run: `I:\ComfyUI\repair_directml.bat`

Then launch again from Stability Matrix only (not Comfy Desktop + bat at same time).

## Finish model migration (slow HDD — use SuperCopy)

Robocopy often hangs on the slow I: drive. Use **SuperCopy** instead:

```powershell
# All remaining categories (overnight)
pwsh -NoProfile -File D:\repos\archhive-app\scripts\migrate-models-supercopy.ps1

# One category (e.g. moody ~59 GB)
pwsh -NoProfile -File D:\repos\archhive-app\scripts\migrate-models-supercopy.ps1 -Category moody -Workers 6
```

Build SuperCopy once if needed: `cd D:\repos\supercopy && npm run build`

Legacy wrapper (migration + junctions): `setup-ai-env-continue.ps1`

## Environment (already set)

- HF_HOME = I:\huggingface
- OLLAMA_MODELS = I:\Models
- PIP_CACHE_DIR = I:\pip-cache
- TEMP/TMP = D:\temp (faster than I: HDD)

## Do not use daily

- Comfy Desktop (backup only)
- run_comfy_headless.bat (emergency only)

## RX 6600 tips

- Use FP8/GGUF models when possible (8 GB VRAM)
- Keep `--lowvram --directml` launch args
- Avoid running two launchers on port 8188
