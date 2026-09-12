$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "mcp-smoke-support.ps1")

$command = $env:GETCODEXY_CANDIDATE_RUNTIME
if ([string]::IsNullOrWhiteSpace($command)) { throw "candidate getcodexy runtime executable is not configured" }
if (-not (Test-Path -LiteralPath $command -PathType Leaf)) { throw "candidate getcodexy runtime executable is missing: $command" }
$pluginRoot = (Resolve-Path -LiteralPath "plugins/codexy").Path
$runtime = Join-Path $env:CODEXY_RUNTIME_DIR "codexy-mcp-watcher-windows-x86_64.exe"
if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) { throw "verified public Watcher runtime is missing: $runtime" }

$result = Invoke-McpProtocol `
  -FileName $command `
  -Arguments @("watcher", "--plugin-root", $pluginRoot, "--", "--stdio") `
  -WorkingDirectory $pluginRoot `
  -Server "watcher" `
  -Environment @{ CODEXY_RUNTIME_DIR = $env:CODEXY_RUNTIME_DIR } `
  -ClientName "candidate-wheel-windows-smoke"
Write-Host (@{
    command = $command
    runtime = $runtime
    initialize = "ok"
    version = $result.Initialize.result.serverInfo.version
  } | ConvertTo-Json -Compress)
