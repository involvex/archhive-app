# Set up NDK toolchain environment variables for cross-compilation.
#
# The Tauri CLI does not set CC/AR env vars for the Android NDK Clang on
# Windows, so cc-rs (used by aws-lc / rustls) cannot find the compiler
# or archiver and cargo exits with code -1.
#
# Source this script from the same PowerShell session that runs
# `tauri android build` — do NOT invoke with `pwsh -File` as a child
# process (env vars won't propagate to the parent).

# Detect NDK root path.
$ndkPath = $env:ANDROID_NDK_HOME
if (-not $ndkPath) { $ndkPath = $env:ANDROID_NDK_ROOT }
if (-not $ndkPath) {
    $sdkRoot = $env:ANDROID_SDK_ROOT
    if (-not $sdkRoot) { $sdkRoot = $env:ANDROID_HOME }
    if ($sdkRoot) {
        $ndkDir = Join-Path $sdkRoot "ndk"
        if (Test-Path $ndkDir) {
            $latest = Get-ChildItem $ndkDir -Directory | Sort-Object Name -Descending | Select-Object -First 1
            $ndkPath = Join-Path $ndkDir $latest.Name
        }
    }
}

if ($ndkPath -and (Test-Path $ndkPath)) {
    $ndkBin = Join-Path $ndkPath "toolchains\llvm\prebuilt\windows-x86_64\bin"
    $clang = Join-Path $ndkBin "clang.exe"
    $ar = Join-Path $ndkBin "llvm-ar.exe"
    if ((Test-Path $clang) -and (Test-Path $ar)) {
        [System.Environment]::SetEnvironmentVariable("CC_aarch64-linux-android", $clang)
        [System.Environment]::SetEnvironmentVariable("AR_aarch64-linux-android", $ar)
        Write-Host "NDK toolchain configured:"
        Write-Host "  CC=$clang"
        Write-Host "  AR=$ar"
    } else {
        Write-Warning "NDK Clang/LLVM-AR not found at $ndkBin — Android native compilation may fail"
    }
} else {
    Write-Warning "NDK path could not be resolved (set ANDROID_NDK_HOME or ANDROID_NDK_ROOT) — Android native compilation may fail"
}
