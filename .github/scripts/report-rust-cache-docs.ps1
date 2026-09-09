param([string]$Suite)
if ($Suite -ne "library and binaries") { return }
$ErrorActionPreference = "Stop"
$docs = Join-Path (Join-Path $HOME $env:CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH) "share/doc"
$exists = Test-Path -LiteralPath $docs -PathType Container
$files = if ($exists) { @(Get-ChildItem -LiteralPath $docs -Recurse -File) } else { @() }
$bytes = ($files | Measure-Object -Property Length -Sum).Sum
Write-Output "normal-cache-excluded-docs exists=$exists files=$($files.Count) bytes=$([long]$bytes)"
