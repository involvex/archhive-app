# Wrapper for `tauri android build` that sets up NDK toolchain env vars.
#
# The Tauri CLI does not set CC/AR env vars for the Android NDK Clang,
# causing cc-rs (used by aws-lc / rustls) to fail finding the compiler
# or archiver and cargo exits with code -1.
#
# This script sources setup-ndk-env.ps1 in the same process, then forwards
# all arguments to `bun run tauri android build`.

. "$PSScriptRoot\setup-ndk-env.ps1"
& bun run tauri android build $args
