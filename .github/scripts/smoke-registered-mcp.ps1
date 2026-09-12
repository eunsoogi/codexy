$ErrorActionPreference = "Stop"
$cacheRoot = Join-Path $env:RUNNER_TEMP "codexy-mcp-registration-cache"
$env:CODEXY_RUNTIME_CACHE_DIR = $cacheRoot
New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null

function Read-McpResponse {
  param(
    [System.Diagnostics.Process] $Process,
    [string] $Server,
    [int] $TimeoutMs = 30000
  )
  $read = $Process.StandardOutput.ReadLineAsync()
  if (-not $read.Wait($TimeoutMs)) { throw "registered MCP response timed out: $Server" }
  $line = $read.Result
  if ([string]::IsNullOrWhiteSpace($line)) { throw "registered MCP server closed stdout: $Server" }
  return $line | ConvertFrom-Json
}

function Send-McpRequest {
  param(
    [System.Diagnostics.Process] $Process,
    [hashtable] $Request,
    [string] $Server
  )
  $Process.StandardInput.WriteLine(($Request | ConvertTo-Json -Compress -Depth 20))
  $Process.StandardInput.Flush()
  return Read-McpResponse -Process $Process -Server $Server
}

function Invoke-McpRegistration {
  param(
    [string] $PluginRoot,
    [string] $Server,
    [int] $ExpectedTools
  )
  $resolvedRoot = (Resolve-Path -LiteralPath $PluginRoot).Path
  $config = Get-Content -Raw -LiteralPath (Join-Path $resolvedRoot ".mcp.json") | ConvertFrom-Json
  $entry = $config.PSObject.Properties[$Server].Value
  if ($null -eq $entry) { throw "registered MCP server is missing: $Server" }
  $command = [string]$entry.command
  if ([string]::IsNullOrWhiteSpace($command)) { throw "registered MCP command is empty: $Server" }
  Get-Command -Name $command -ErrorAction Stop | Out-Null

  $start = [System.Diagnostics.ProcessStartInfo]::new()
  $start.FileName = $command
  $start.WorkingDirectory = $resolvedRoot
  $start.UseShellExecute = $false
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in @($entry.args)) { $start.ArgumentList.Add([string]$argument) }
  $process = [System.Diagnostics.Process]::new()
  $process.StartInfo = $start
  $started = $false
  $stderrTask = $null
  try {
    if (-not $process.Start()) { throw "failed to start registered MCP server: $Server" }
    $started = $true
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $initializeRequest = @{
      jsonrpc = "2.0"
      id = 1
      method = "initialize"
      params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{
          name = "python-package-windows-smoke"
          version = "1"
        }
      }
    }
    $initialize = Send-McpRequest -Process $process -Request $initializeRequest -Server $Server
    if ($initialize.error -or $initialize.result.protocolVersion -ne "2024-11-05") { throw "registered MCP initialize failed: $Server" }
    $initialized = @{ jsonrpc = "2.0"; method = "notifications/initialized" }
    $process.StandardInput.WriteLine(($initialized | ConvertTo-Json -Compress -Depth 20))
    $process.StandardInput.Flush()
    $listed = Send-McpRequest -Process $process -Request @{ jsonrpc = "2.0"; id = 2; method = "tools/list"; params = @{} } -Server $Server
    if ($listed.error) { throw "registered MCP tools/list failed: $Server" }
    $tools = @($listed.result.tools)
    if ($tools.Count -ne $ExpectedTools) { throw "registered MCP tool count for $Server was $($tools.Count), expected $ExpectedTools" }
    Write-Host (@{ server = $Server; command = $command; args = @($entry.args); initialize = "ok"; tools = $tools.Count } | ConvertTo-Json -Compress -Depth 20)
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(30000)) { throw "registered MCP server timed out: $Server" }
    if ($null -ne $stderrTask -and -not $stderrTask.Wait(5000)) { throw "registered MCP stderr drain timed out: $Server" }
    if ($process.ExitCode -ne 0) { throw "registered MCP server exited $($process.ExitCode): $Server`n$($stderrTask.Result)" }
  } finally {
    if ($started -and -not $process.HasExited) { $process.Kill($true); $process.WaitForExit() }
    $process.Dispose()
  }
}

Invoke-McpRegistration "plugins/codexy" watcher 5
Invoke-McpRegistration "plugins/codexy-devtools" lsp 8
Invoke-McpRegistration "plugins/codexy-devtools" codegraph 6
