$ErrorActionPreference = "Stop"

$pluginRoot = (Resolve-Path -LiteralPath "plugins/codexy").Path
$bundleRoot = Join-Path $env:RUNNER_TEMP "codexy-public-marketplace-bundle"
$manifest = Get-Content -Raw -LiteralPath (Join-Path $pluginRoot ".codex-plugin/plugin.json") | ConvertFrom-Json
$version = [string]$manifest.version
$runtime = Join-Path $bundleRoot "extracted/plugins/codexy/runtime/codexy-mcp-watcher-windows-x86_64.exe"
if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) { throw "verified public Watcher runtime is missing: $runtime" }

$cacheRoot = Join-Path $env:RUNNER_TEMP "codexy-mcp-bootstrap-diagnostic-cache"
$stateRoot = Join-Path $env:RUNNER_TEMP "codexy-mcp-bootstrap-diagnostic-state"
New-Item -ItemType Directory -Force -Path $cacheRoot, $stateRoot | Out-Null
$env:CODEXY_RUNTIME_CACHE_DIR = $cacheRoot
$env:CODEXY_WATCHER_STATE_DIR = $stateRoot
$env:CODEXY_PLUGIN_ROOT = $pluginRoot
Remove-Item Env:CODEXY_RUNTIME_DIR -ErrorAction SilentlyContinue

$request = @{
  jsonrpc = "2.0"
  id = 1
  method = "initialize"
  params = @{
    protocolVersion = "2024-11-05"
    capabilities = @{}
    clientInfo = @{ name = "mcp-bootstrap-diagnostic"; version = "1" }
  }
} | ConvertTo-Json -Compress -Depth 10

function Invoke-DiagnosticProcess {
  param(
    [string] $Name,
    [string] $FileName,
    [string[]] $Arguments,
    [string] $WorkingDirectory,
    [string] $InputText,
    [int] $TimeoutMs = 30000
  )
  $start = [System.Diagnostics.ProcessStartInfo]::new()
  $start.FileName = $FileName
  $start.WorkingDirectory = $WorkingDirectory
  $start.UseShellExecute = $false
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in $Arguments) { $start.ArgumentList.Add([string]$argument) }
  $process = [System.Diagnostics.Process]::new()
  $process.StartInfo = $start
  $stdoutTask = $null
  $stderrTask = $null
  $stdout = ""
  $stderr = ""
  $exitCode = -1
  $timedOut = $false
  try {
    if (-not $process.Start()) { throw "process did not start" }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if ($null -ne $InputText) {
      $process.StandardInput.WriteLine($InputText)
    }
    $process.StandardInput.Close()
    if (-not $process.WaitForExit($TimeoutMs)) {
      $timedOut = $true
      try { $process.Kill($true) } catch { }
      $process.WaitForExit(5000) | Out-Null
    }
    if ($null -ne $stdoutTask -and $stdoutTask.Wait(5000)) { $stdout = [string]$stdoutTask.Result }
    if ($null -ne $stderrTask -and $stderrTask.Wait(5000)) { $stderr = [string]$stderrTask.Result }
    if ($process.HasExited) { $exitCode = $process.ExitCode }
  } catch {
    $stderr = $_.Exception.Message
  } finally {
    $process.Dispose()
  }
  [pscustomobject]@{
    name = $Name
    exit_code = $exitCode
    timed_out = $timedOut
    stdout = (($stdout -replace "\s+", " ").Trim())
    stderr = (($stderr -replace "\s+", " ").Trim())
  }
}

$uvScript = Invoke-DiagnosticProcess `
  "uv-python-script" `
  "uv" `
  @("run", "--no-project", "--script", "./mcp/codexy_mcp_bootstrap.py", "invalid") `
  $pluginRoot `
  $null
$uvxHelp = Invoke-DiagnosticProcess `
  "uvx-package-entrypoint" `
  "uvx" `
  @("--from", "getcodexy==$version", "codexy-mcp-runtime", "--help") `
  $pluginRoot `
  $null `
  120000
$native = Invoke-DiagnosticProcess `
  "native-runtime" `
  $runtime `
  @("--stdio") `
  $pluginRoot `
  $request
$packageRuntime = Invoke-DiagnosticProcess `
  "uvx-runtime-default" `
  "uvx" `
  @("--from", "getcodexy==$version", "codexy-mcp-runtime", "watcher", "--plugin-root", $pluginRoot, "--", "--stdio") `
  $pluginRoot `
  $request `
  120000
$registered = Invoke-DiagnosticProcess `
  "registered-bootstrap-default" `
  "uv" `
  @("run", "--no-project", "--script", "./mcp/codexy_mcp_bootstrap.py", "watcher", "--stdio") `
  $pluginRoot `
  $request `
  120000

$results = @($uvScript, $uvxHelp, $native, $packageRuntime, $registered)
foreach ($result in $results) {
  Write-Host ($result | ConvertTo-Json -Compress)
}

$expectations = @(
  @{ result = $uvScript; code = 64; marker = "codexy_mcp_bootstrap requires" },
  @{ result = $uvxHelp; code = 0; marker = "usage:" },
  @{ result = $native; code = 0; marker = '"protocolVersion":"2024-11-05"' },
  @{ result = $packageRuntime; code = 0; marker = '"protocolVersion":"2024-11-05"' },
  @{ result = $registered; code = 0; marker = '"protocolVersion":"2024-11-05"' }
)
$failures = @(
  $expectations | Where-Object {
    $_.result.exit_code -ne $_.code -or
    ("$($_.result.stdout) $($_.result.stderr)" -notlike "*$($_.marker)*") -or
    $_.result.timed_out
  }
)
if ($failures.Count -gt 0) {
  throw "MCP bootstrap stage diagnostics failed: $($failures.result.name -join ', ')"
}
