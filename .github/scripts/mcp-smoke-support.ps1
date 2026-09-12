function Read-McpResponse {
  param(
    [System.Diagnostics.Process] $Process,
    [string] $Server,
    [int] $TimeoutMs = 30000
  )
  $read = $Process.StandardOutput.ReadLineAsync()
  if (-not $read.Wait($TimeoutMs)) { throw "MCP response timed out: $Server" }
  $line = $read.Result
  if ([string]::IsNullOrWhiteSpace($line)) { throw "MCP server closed stdout: $Server" }
  return $line | ConvertFrom-Json
}

function Get-McpProcessDiagnostics {
  param(
    [System.Diagnostics.Process] $Process,
    [object] $StderrTask
  )
  $exit = "running"
  try {
    if ($Process.HasExited) { $exit = [string]$Process.ExitCode }
  } catch {
    $exit = "unavailable"
  }
  $stderr = "<not-collected>"
  if ($null -ne $StderrTask) {
    if (-not $StderrTask.IsCompleted) {
      try { $StderrTask.Wait(5000) | Out-Null } catch { }
    }
    if ($StderrTask.IsCompleted) {
      try { $stderr = [string]$StderrTask.Result } catch { $stderr = "<read-failed>" }
    } else {
      $stderr = "<pending>"
    }
  }
  $stderr = ($stderr -replace "\s+", " ").Trim()
  if ([string]::IsNullOrWhiteSpace($stderr)) { $stderr = "<empty>" }
  return "exit=$exit; stderr=$stderr"
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

function Send-McpNotification {
  param(
    [System.Diagnostics.Process] $Process,
    [hashtable] $Notification
  )
  $Process.StandardInput.WriteLine(($Notification | ConvertTo-Json -Compress -Depth 20))
  $Process.StandardInput.Flush()
}

function Invoke-McpProtocol {
  param(
    [Parameter(Mandatory = $true)] [string] $FileName,
    [Parameter(Mandatory = $true)] [string[]] $Arguments,
    [Parameter(Mandatory = $true)] [string] $WorkingDirectory,
    [Parameter(Mandatory = $true)] [string] $Server,
    [hashtable] $Environment = @{},
    [int] $ExpectedTools = -1,
    [string] $ClientName = "codexy-mcp-smoke"
  )
  $start = [System.Diagnostics.ProcessStartInfo]::new()
  $start.FileName = $FileName
  $start.WorkingDirectory = $WorkingDirectory
  $start.UseShellExecute = $false
  $start.RedirectStandardInput = $true
  $start.RedirectStandardOutput = $true
  $start.RedirectStandardError = $true
  foreach ($argument in $Arguments) { $start.ArgumentList.Add([string]$argument) }
  foreach ($key in @("CODEXY_RUNTIME_DIR", "CODEXY_RUNTIME_PACKAGE_PATH", "CODEXY_RUNTIME_PACKAGE_URL", "CODEXY_RUNTIME_ARTIFACTS_API_URL", "CODEXY_RUNTIME_PACKAGE_SHA256")) {
    [void]$start.Environment.Remove($key)
  }
  foreach ($key in $Environment.Keys) { $start.Environment[$key] = [string]$Environment[$key] }
  $process = [System.Diagnostics.Process]::new()
  $process.StartInfo = $start
  $started = $false
  $stderrTask = $null
  try {
    if (-not $process.Start()) { throw "MCP server did not start: $Server" }
    $started = $true
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $initialize = Send-McpRequest -Process $process -Server $Server -Request @{
      jsonrpc = "2.0"
      id = 1
      method = "initialize"
      params = @{
        protocolVersion = "2024-11-05"
        capabilities = @{}
        clientInfo = @{ name = $ClientName; version = "1" }
      }
    }
    if ($initialize.error -or $initialize.result.protocolVersion -ne "2024-11-05") {
      throw "MCP initialize failed: $Server"
    }
    $tools = $null
    if ($ExpectedTools -ge 0) {
      Send-McpNotification -Process $process -Notification @{ jsonrpc = "2.0"; method = "notifications/initialized" }
      $listed = Send-McpRequest -Process $process -Server $Server -Request @{ jsonrpc = "2.0"; id = 2; method = "tools/list"; params = @{} }
      if ($listed.error) { throw "MCP tools/list failed: $Server" }
      $tools = @($listed.result.tools)
      if ($tools.Count -ne $ExpectedTools) { throw "MCP tool count for $Server was $($tools.Count), expected $ExpectedTools" }
    }
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(30000)) { throw "MCP server timed out: $Server" }
    if ($null -ne $stderrTask -and -not $stderrTask.Wait(5000)) { throw "MCP stderr drain timed out: $Server" }
    if ($process.ExitCode -ne 0) { throw "MCP server exited $($process.ExitCode): $Server" }
    return [pscustomobject]@{
      Initialize = $initialize
      Tools = $tools
      FileName = $FileName
      Arguments = @($Arguments)
      ExitCode = $process.ExitCode
    }
  } catch {
    if ($started) {
      try {
        if (-not $process.HasExited) { $process.Kill($true) }
      } catch { }
      try { $process.WaitForExit(5000) | Out-Null } catch { }
      throw "MCP server failed: ${Server}; $($_.Exception.Message); $(Get-McpProcessDiagnostics -Process $process -StderrTask $stderrTask)"
    }
    throw
  } finally {
    if ($started) {
      try {
        if (-not $process.HasExited) { $process.Kill($true); $process.WaitForExit(5000) | Out-Null }
      } catch { }
      $process.Dispose()
    }
  }
}
