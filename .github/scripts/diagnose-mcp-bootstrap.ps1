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
$cachedFiles = @(
  Get-ChildItem -LiteralPath $cacheRoot -Recurse -File -Filter "codexy-mcp-watcher.exe" |
    Where-Object { $_.Directory.Name -eq "bin" }
)
$cachedRuntime = $null
$cacheObservation = [pscustomobject]@{
  name = "cached-runtime-bytes"
  path = ""
  verified_sha256 = ""
  cached_sha256 = ""
  same_bytes = $false
}
if ($cachedFiles.Count -eq 1) {
  $cachedRuntime = $cachedFiles[0].FullName
  $verifiedSha = (Get-FileHash -LiteralPath $runtime -Algorithm SHA256).Hash.ToLowerInvariant()
  $cachedSha = (Get-FileHash -LiteralPath $cachedRuntime -Algorithm SHA256).Hash.ToLowerInvariant()
  $cacheObservation = [pscustomobject]@{
    name = "cached-runtime-bytes"
    path = $cachedRuntime
    verified_sha256 = $verifiedSha
    cached_sha256 = $cachedSha
    same_bytes = $verifiedSha -eq $cachedSha
  }
}
if ($null -ne $cachedRuntime) {
  $cachedDirect = Invoke-DiagnosticProcess `
    "cached-native-runtime" `
    $cachedRuntime `
    @("--stdio") `
    $pluginRoot `
    $request
  $execHandoffCode = 'import os,sys; command=sys.argv[1]; os.execvpe(command, [command, *sys.argv[2:]], os.environ)'
  $uvxExecHandoff = Invoke-DiagnosticProcess `
    "uvx-execvpe-native" `
    "uvx" `
    @("--from", "getcodexy==$version", "python", "-c", $execHandoffCode, $cachedRuntime, "--stdio") `
    $pluginRoot `
    $request `
    120000
  $subprocessHandoffCode = 'import subprocess,sys; raise SystemExit(subprocess.run([sys.argv[1], *sys.argv[2:]], check=False).returncode)'
  $uvxSubprocessHandoff = Invoke-DiagnosticProcess `
    "uvx-subprocess-native" `
    "uvx" `
    @("--from", "getcodexy==$version", "python", "-c", $subprocessHandoffCode, $cachedRuntime, "--stdio") `
    $pluginRoot `
    $request `
    120000
} else {
  $cachedDirect = [pscustomobject]@{ name = "cached-native-runtime"; exit_code = -1; timed_out = $false; stdout = ""; stderr = "cached runtime was not materialized" }
  $uvxExecHandoff = [pscustomobject]@{ name = "uvx-execvpe-native"; exit_code = -1; timed_out = $false; stdout = ""; stderr = "cached runtime was not materialized" }
  $uvxSubprocessHandoff = [pscustomobject]@{ name = "uvx-subprocess-native"; exit_code = -1; timed_out = $false; stdout = ""; stderr = "cached runtime was not materialized" }
}
$registered = Invoke-DiagnosticProcess `
  "registered-bootstrap-default" `
  "uv" `
  @("run", "--no-project", "--script", "./mcp/codexy_mcp_bootstrap.py", "watcher", "--stdio") `
  $pluginRoot `
  $request `
  120000
$registeredSubprocessCode = 'import os,subprocess,sys; root,version=sys.argv[1:3]; command=["uvx","--from",f"getcodexy=={version}","codexy-mcp-runtime","watcher","--plugin-root",root,"--","--stdio"]; environment=os.environ.copy(); environment["CODEXY_PLUGIN_ROOT"]=root; raise SystemExit(subprocess.run(command, env=environment, check=False).returncode)'
$registeredSubprocess = Invoke-DiagnosticProcess `
  "registered-bootstrap-subprocess" `
  "uv" `
  @("run", "--no-project", "python", "-c", $registeredSubprocessCode, $pluginRoot, $version) `
  $pluginRoot `
  $request `
  120000

$results = @($uvScript, $uvxHelp, $native, $packageRuntime, $cacheObservation, $cachedDirect, $uvxExecHandoff, $uvxSubprocessHandoff, $registered, $registeredSubprocess)
foreach ($result in $results) {
  Write-Host ($result | ConvertTo-Json -Compress)
}

$expectations = @(
  [pscustomobject]@{ name = "uv-python-script"; ok = $uvScript.exit_code -eq 64 -and "$($uvScript.stdout) $($uvScript.stderr)" -like "*codexy_mcp_bootstrap requires*" -and -not $uvScript.timed_out },
  [pscustomobject]@{ name = "uvx-package-entrypoint"; ok = $uvxHelp.exit_code -eq 0 -and "$($uvxHelp.stdout) $($uvxHelp.stderr)" -like "*usage:*" -and -not $uvxHelp.timed_out },
  [pscustomobject]@{ name = "native-runtime"; ok = $native.exit_code -eq 0 -and "$($native.stdout) $($native.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $native.timed_out },
  [pscustomobject]@{ name = "uvx-runtime-default"; ok = $packageRuntime.exit_code -eq 0 -and "$($packageRuntime.stdout) $($packageRuntime.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $packageRuntime.timed_out },
  [pscustomobject]@{ name = "cached-runtime-bytes"; ok = $cacheObservation.same_bytes },
  [pscustomobject]@{ name = "cached-native-runtime"; ok = $cachedDirect.exit_code -eq 0 -and "$($cachedDirect.stdout) $($cachedDirect.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $cachedDirect.timed_out },
  [pscustomobject]@{ name = "uvx-execvpe-native"; ok = $uvxExecHandoff.exit_code -eq 0 -and "$($uvxExecHandoff.stdout) $($uvxExecHandoff.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $uvxExecHandoff.timed_out },
  [pscustomobject]@{ name = "uvx-subprocess-native"; ok = $uvxSubprocessHandoff.exit_code -eq 0 -and "$($uvxSubprocessHandoff.stdout) $($uvxSubprocessHandoff.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $uvxSubprocessHandoff.timed_out },
  [pscustomobject]@{ name = "registered-bootstrap-default"; ok = $registered.exit_code -eq 0 -and "$($registered.stdout) $($registered.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $registered.timed_out },
  [pscustomobject]@{ name = "registered-bootstrap-subprocess"; ok = $registeredSubprocess.exit_code -eq 0 -and "$($registeredSubprocess.stdout) $($registeredSubprocess.stderr)" -like '*"protocolVersion":"2024-11-05"*' -and -not $registeredSubprocess.timed_out }
)
$failures = @($expectations | Where-Object { -not $_.ok })
if ($failures.Count -gt 0) {
  throw "MCP bootstrap stage diagnostics failed: $($failures.name -join ', ')"
}
