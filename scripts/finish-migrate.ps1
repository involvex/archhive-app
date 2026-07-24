param(
    [string]$Category = 'checkpoints',
    [int]$Workers = 4
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'lib\supercopy-migrate.ps1')

Invoke-SuperCopyDir `
    -Source "I:\ComfyUI\ComfyUI\models\$Category" `
    -DestParent 'I:\Models' `
    -Move `
    -Workers $Workers `
    -LogName $Category

Write-Host "done $Category"
