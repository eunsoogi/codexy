$ErrorActionPreference = "Stop"

$command = $env:GETCODEXY_CANDIDATE_RUNTIME
if ([string]::IsNullOrWhiteSpace($command)) { throw "candidate getcodexy runtime executable is not configured" }
if (-not (Test-Path -LiteralPath $command -PathType Leaf)) { throw "candidate getcodexy runtime executable is missing: $command" }
$pluginRoot = (Resolve-Path -LiteralPath "plugins/codexy").Path
$runtime = Join-Path $env:CODEXY_RUNTIME_DIR "codexy-mcp-watcher-windows-x86_64.exe"
if (-not (Test-Path -LiteralPath $runtime -PathType Leaf)) { throw "verified public Watcher runtime is missing: $runtime" }

$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = $command
$start.WorkingDirectory = $pluginRoot
$start.UseShellExecute = $false
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
foreach ($argument in @("watcher", "--plugin-root", $pluginRoot, "--", "--stdio")) {
  $start.ArgumentList.Add($argument)
}
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $start
$started = $false
$request = @{
  jsonrpc = "2.0"
  id = 1
  method = "initialize"
  params = @{
    protocolVersion = "2024-11-05"
    capabilities = @{}
    clientInfo = @{ name = "candidate-wheel-windows-smoke"; version = "1" }
  }
} | ConvertTo-Json -Compress -Depth 10
try {
  if (-not $process.Start()) { throw "candidate runtime did not start" }
  $started = $true
  $stdoutTask = $process.StandardOutput.ReadToEndAsync()
  $stderrTask = $process.StandardError.ReadToEndAsync()
  $process.StandardInput.WriteLine($request)
  $process.StandardInput.Close()
  if (-not $process.WaitForExit(30000)) {
    try { $process.Kill($true) } catch { }
    throw "candidate runtime timed out"
  }
  $stdout = if ($stdoutTask.Wait(5000)) { [string]$stdoutTask.Result } else { "" }
  $stderr = if ($stderrTask.Wait(5000)) { [string]$stderrTask.Result } else { "" }
  $response = $stdout | ConvertFrom-Json
  if ($process.ExitCode -ne 0) { throw "candidate runtime exited $($process.ExitCode): $stderr" }
  if ($response.result.protocolVersion -ne "2024-11-05") { throw "candidate runtime returned an unexpected MCP protocol" }
  Write-Host (@{
      command = $command
      runtime = $runtime
      initialize = "ok"
      version = $response.result.serverInfo.version
    } | ConvertTo-Json -Compress)
} finally {
  if ($started -and $process.HasExited -eq $false) {
    try { $process.Kill($true); $process.WaitForExit(5000) | Out-Null } catch { }
  }
  $process.Dispose()
}
