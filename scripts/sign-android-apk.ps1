# Zipalign + sign a release APK so Android will install it (Windows / pwsh).
# Same behaviour as scripts/sign-android-apk.sh, which CI uses.
#
# `tauri android build` (release, no signingConfig) emits
# app-universal-release-unsigned.apk; Android refuses unsigned APKs
# ("App not installed" / "package appears to be invalid").
#
# Usage:
#   pwsh -NoProfile -File scripts/sign-android-apk.ps1 -In <unsigned.apk> -Out <signed.apk>
#   (-In defaults to the universal release APK under src-tauri/gen/android)
#
# Key (stable across releases, so updates install over each other):
#   -Keystore <path.jks> -KeyAlias <alias>, or env ANDROID_KEYSTORE_BASE64 + ANDROID_KEY_ALIAS
#   Passwords: env ANDROID_KEYSTORE_PASSWORD and ANDROID_KEY_PASSWORD
#   (key password defaults to the keystore password).
# Without a key a throwaway one is generated: the APK installs, but a later
# build signed with a different key cannot update it (uninstall first).

param(
    [string]$In = (Join-Path $PSScriptRoot "..\src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release-unsigned.apk"),
    [string]$Out = (Join-Path $PSScriptRoot "..\ArcHive-android.apk"),
    [string]$Keystore = "",
    [string]$KeyAlias = ""
)

$ErrorActionPreference = "Stop"

# Load repo-root .env (local signing secrets) for vars not already set.
# CI passes the same values as encrypted secrets instead.
$dotEnv = Join-Path $PSScriptRoot "..\.env"
if (Test-Path $dotEnv) {
    foreach ($line in (Get-Content $dotEnv -Encoding ascii)) {
        if ($line -match '^\s*#' -or $line -notmatch '^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)\s*$') { continue }
        $name, $value = $Matches[1], $Matches[2]
        if (-not (Test-Path "env:$name") -or [string]::IsNullOrEmpty((Get-Item "env:$name").Value)) {
            Set-Item -Path "env:$name" -Value $value
        }
    }
}

if (-not (Test-Path $In)) { throw "APK not found: $In" }

$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } elseif ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else {
    Join-Path $env:LOCALAPPDATA "Android\Sdk"
}
$btRoot = Join-Path $sdk "build-tools"
if (-not (Test-Path $btRoot)) { throw "Android build-tools not found under ${sdk}. Set ANDROID_HOME." }
$bt = Get-ChildItem $btRoot -Directory |
    Sort-Object { try { [version]$_.Name } catch { [version]"0.0" } } |
    Select-Object -Last 1
if (-not $bt) { throw "No build-tools versions found in $btRoot" }
$isWin = $IsWindows -or ($env:OS -eq "Windows_NT")
$zipalign = Join-Path $bt.FullName ($(if ($isWin) { "zipalign.exe" } else { "zipalign" }))
$apksigner = Join-Path $bt.FullName ($(if ($isWin) { "apksigner.bat" } else { "apksigner" }))
foreach ($tool in @($zipalign, $apksigner)) {
    if (-not (Test-Path $tool)) { throw "Missing ${tool}. Install build-tools with the Android SDK manager." }
}

$keytool = "keytool"
if ($env:JAVA_HOME) {
    $candidate = Join-Path $env:JAVA_HOME ($(if ($isWin) { "bin\keytool.exe" } else { "bin/keytool" }))
    if (Test-Path $candidate) { $keytool = $candidate }
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("archhive-sign-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $storePass = $env:ANDROID_KEYSTORE_PASSWORD
    $keyPass = if ($env:ANDROID_KEY_PASSWORD) { $env:ANDROID_KEY_PASSWORD } else { $storePass }
    $alias = if ($KeyAlias) { $KeyAlias } else { $env:ANDROID_KEY_ALIAS }

    if (-not $Keystore -and $env:ANDROID_KEYSTORE_BASE64) {
        $Keystore = Join-Path $work "release.jks"
        [System.IO.File]::WriteAllBytes($Keystore, [Convert]::FromBase64String($env:ANDROID_KEYSTORE_BASE64))
    }

    if ($Keystore) {
        if (-not (Test-Path $Keystore)) { throw "Keystore not found: $Keystore" }
        if (-not $storePass) { throw "Set ANDROID_KEYSTORE_PASSWORD for the keystore." }
        if (-not $alias) { throw "Pass -KeyAlias or set ANDROID_KEY_ALIAS." }
        Write-Host "Signing with the release key (alias $alias)."
    } else {
        $Keystore = Join-Path $work "throwaway.jks"
        $storePass = [guid]::NewGuid().ToString("N")
        $keyPass = $storePass
        $alias = "archhive"
        & $keytool -genkeypair -keystore $Keystore -storepass $storePass -keypass $keyPass `
            -alias $alias -keyalg RSA -keysize 2048 -validity 10000 `
            -dname "CN=ArcHive (unofficial build)" 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "keytool failed (is a JDK on PATH or JAVA_HOME set?)" }
        Write-Warning "No release key: signed with a throwaway key. The APK installs, but cannot update an install signed with another key."
    }

    $aligned = Join-Path $work "aligned.apk"
    # -P 16: 16 KB page alignment for uncompressed .so files (Android 15+ devices).
    & $zipalign -P 16 -f 4 $In $aligned
    if ($LASTEXITCODE -ne 0) { throw "zipalign failed" }

    $storePassFile = Join-Path $work "store.pass"
    $keyPassFile = Join-Path $work "key.pass"
    [System.IO.File]::WriteAllText($storePassFile, $storePass)
    [System.IO.File]::WriteAllText($keyPassFile, $keyPass)
    $outDir = Split-Path -Parent ([System.IO.Path]::GetFullPath($Out))
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null

    & $apksigner sign --ks $Keystore --ks-key-alias $alias `
        --ks-pass "file:$storePassFile" --key-pass "file:$keyPassFile" `
        --out $Out $aligned
    if ($LASTEXITCODE -ne 0) { throw "apksigner sign failed" }
    & $apksigner verify --print-certs $Out | Select-Object -First 2
    if ($LASTEXITCODE -ne 0) { throw "apksigner verify failed" }
    & $zipalign -c -P 16 4 $Out | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "zipalign check failed" }
    Write-Host "Signed + aligned: $Out"
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
