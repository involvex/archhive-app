# Bump version, optionally build desktop + Android APK, commit, tag, and push.
# GitHub Actions (.github/workflows/release.yml) builds official artifacts on tag push.

param(
  [Parameter(Mandatory = $true)]
  [string]$Version,
  [switch]$BuildLocal,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Semver = '^\d+\.\d+\.\d+(-[\w.-]+)?$'
if ($Version -notmatch $Semver) {
  throw "Version must be semver (e.g. 0.2.0), got: $Version"
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$tag = "v$Version"
Write-Host "Release $tag"

Write-Host "Bumping version files..."
if ($DryRun) {
  Write-Host "[dry-run] bun scripts/bump-version.ts $Version"
} else {
  bun scripts/bump-version.ts $Version
}

Write-Host "Regenerating plugins registry..."
if ($DryRun) {
  Write-Host "[dry-run] bun run plugins:generate"
} else {
  bun run plugins:generate
}

if ($BuildLocal) {
  Write-Host "Applying Android launcher icons..."
  if (-not $DryRun) {
    $icon = Join-Path $root "assets\branding\icon-source.png"
    if (Test-Path $icon) {
      bun run tauri icon $icon
    }
  }

  Write-Host "Building desktop installer..."
  if ($DryRun) {
    Write-Host "[dry-run] bun run build:desktop"
  } else {
    bun run build:desktop
  }

  Write-Host "Building Android release APK..."
  if ($DryRun) {
    Write-Host "[dry-run] bun run build:apk:release"
  } else {
    bun run build:apk:release
  }
}

$commitMsg = "chore: release $tag"
Write-Host "Commit: $commitMsg"
if ($DryRun) {
  Write-Host "[dry-run] git add -A && git commit && git tag $tag && git push && git push origin $tag"
  exit 0
}

git add -A
$status = git status --porcelain
if (-not $status) {
  Write-Host "No changes to commit (version already $Version?)"
} else {
  git commit -m $commitMsg
}

$existingTag = git tag -l $tag
if ($existingTag) {
  throw "Tag $tag already exists locally. Delete it or pick a new version."
}

git tag -a $tag -m "Release $tag"
Write-Host "Pushing main and tag $tag ..."
git push origin HEAD
git push origin $tag

Write-Host "Done. GitHub Actions will build the release when tag $tag is received."
