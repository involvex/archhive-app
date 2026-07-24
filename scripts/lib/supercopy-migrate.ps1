# SuperCopy helpers for AI model migration (replaces robocopy on slow I: HDD)
$script:SuperCopyRoot = 'D:\repos\supercopy'

function Get-SuperCopyLauncher {
    $exe = Join-Path $script:SuperCopyRoot 'dist\SuperCopy.exe'
    if (Test-Path -LiteralPath $exe) {
        return @{ Command = $exe; PrefixArgs = @() }
    }

    $py = Join-Path $script:SuperCopyRoot '.venv\Scripts\python.exe'
    $pyScript = Join-Path $script:SuperCopyRoot 'supercopy.py'
    if ((Test-Path -LiteralPath $py) -and (Test-Path -LiteralPath $pyScript)) {
        return @{ Command = $py; PrefixArgs = @($pyScript) }
    }

    $node = Join-Path $script:SuperCopyRoot 'bin\supercopy.js'
    if (Test-Path -LiteralPath $node) {
        return @{ Command = 'node'; PrefixArgs = @($node) }
    }

    throw "SuperCopy not found under $script:SuperCopyRoot. Run: cd D:\repos\supercopy && npm run build"
}

function Invoke-SuperCopyDir {
    <#
    Copy folder Source into DestParent\BaseName(Source) using SuperCopy threading.
    Example: checkpoints + I:\Models -> I:\Models\checkpoints
    #>
    param(
        [Parameter(Mandatory)]
        [string]$Source,
        [Parameter(Mandatory)]
        [string]$DestParent,
        [switch]$Move,
        [switch]$Verify,
        [int]$Workers = 4,
        [string]$LogName = 'supercopy'
    )

    if (-not (Test-Path -LiteralPath $Source)) {
        Write-Host "  skip missing: $Source"
        return
    }

    $fileCount = (Get-ChildItem -LiteralPath $Source -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
    if ($fileCount -eq 0) {
        Write-Host "  skip empty: $Source"
        return
    }

    $destDir = Join-Path $DestParent (Split-Path $Source -Leaf)
    New-Item -ItemType Directory -Path $DestParent -Force | Out-Null

    $launcher = Get-SuperCopyLauncher
    $log = Join-Path 'I:\Models' "_migrate-$LogName.log"
    $args = @($launcher.PrefixArgs) + @(
        $Source,
        $DestParent,
        '-w', "$Workers",
        '-b', '1048576'
    )
    if ($Verify) { $args += '--verify' }

    Write-Host "  SuperCopy: $Source -> $destDir ($fileCount files)"
    $logHeader = "SuperCopy $(Get-Date -Format o)`nSource: $Source`nDestParent: $DestParent`nExpected: $destDir`n"
    Set-Content -Path $log -Value $logHeader -Encoding UTF8

    $prevPyIo = $env:PYTHONIOENCODING
    $env:PYTHONIOENCODING = 'utf-8'
    try {
        & $launcher.Command @args 2>&1 | Tee-Object -FilePath $log -Append
    } finally {
        if ($null -eq $prevPyIo) { Remove-Item Env:PYTHONIOENCODING -ErrorAction SilentlyContinue }
        else { $env:PYTHONIOENCODING = $prevPyIo }
    }

    if ($LASTEXITCODE -ne 0) {
        $destFiles = (Get-ChildItem -LiteralPath $destDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count
        if ($destFiles -ge $fileCount -and $fileCount -gt 0) {
            Write-Host "  SuperCopy exit $LASTEXITCODE but destination has $destFiles/$fileCount files; continuing"
        } else {
            throw "SuperCopy failed for $LogName (exit $LASTEXITCODE). See $log"
        }
    }

    if ($Move -and (Test-Path -LiteralPath $destDir)) {
        Write-Host "  removing source after copy: $Source"
        Remove-Item -LiteralPath $Source -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-SuperCopyFile {
    param(
        [Parameter(Mandatory)]
        [string]$SourceFile,
        [Parameter(Mandatory)]
        [string]$DestDir,
        [switch]$Move
    )

    if (-not (Test-Path -LiteralPath $SourceFile)) { return }
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
    $launcher = Get-SuperCopyLauncher
    $destFile = Join-Path $DestDir (Split-Path $SourceFile -Leaf)
    $args = @($launcher.PrefixArgs) + @($SourceFile, $DestDir, '-w', '2')
    $prevPyIo = $env:PYTHONIOENCODING
    $env:PYTHONIOENCODING = 'utf-8'
    try {
        & $launcher.Command @args
    } finally {
        if ($null -eq $prevPyIo) { Remove-Item Env:PYTHONIOENCODING -ErrorAction SilentlyContinue }
        else { $env:PYTHONIOENCODING = $prevPyIo }
    }
    if ($LASTEXITCODE -ne 0 -and -not (Test-Path -LiteralPath $destFile)) {
        throw "SuperCopy file copy failed: $SourceFile"
    }
    if ($Move) { Remove-Item -LiteralPath $SourceFile -Force -ErrorAction SilentlyContinue }
}

# Only when imported as a module; dot-sourced scripts (finish-migrate.ps1) skip this.
if ($MyInvocation.MyCommand.ScriptBlock.Module) {
    Export-ModuleMember -Function Get-SuperCopyLauncher, Invoke-SuperCopyDir, Invoke-SuperCopyFile
}
