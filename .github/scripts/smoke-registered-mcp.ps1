$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "mcp-smoke-support.ps1")
$temporaryRoot = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) { [System.IO.Path]::GetTempPath() } else { $env:RUNNER_TEMP }
$cacheRoot = Join-Path $temporaryRoot "codexy-mcp-registration-cache"
New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null

$os = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
  "windows"
} elseif ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::OSX)) {
  "darwin"
} else {
  "linux"
}
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
$architecture = if ($architecture -eq "x64") { "x86_64" } else { $architecture }
$platform = "$os-$architecture"
$environment = @{ CODEXY_RUNTIME_CACHE_DIR = $cacheRoot }

function Invoke-McpRegistration {
  param(
    [string] $PluginRoot,
    [string] $Server,
    [int] $ExpectedTools
  )
  $resolvedRoot = (Resolve-Path -LiteralPath $PluginRoot).Path
  $manifest = Get-Content -Raw -LiteralPath (Join-Path $resolvedRoot ".codex-plugin/plugin.json") | ConvertFrom-Json
  if (@($manifest.supportedPlatforms) -notcontains $platform) {
    Write-Host (@{
        plugin = $manifest.name
        server = $Server
        status = "unsupported"
        platform = $platform
        reason = "plugin manifest does not advertise this platform"
      } | ConvertTo-Json -Compress)
    return
  }
  $config = Get-Content -Raw -LiteralPath (Join-Path $resolvedRoot ".mcp.json") | ConvertFrom-Json
  $entry = $config.PSObject.Properties[$Server].Value
  if ($null -eq $entry) { throw "registered MCP server is missing: $Server" }
  $command = [string]$entry.command
  if ([string]::IsNullOrWhiteSpace($command)) { throw "registered MCP command is empty: $Server" }
  Get-Command -Name $command -ErrorAction Stop | Out-Null
  $result = Invoke-McpProtocol -FileName $command -Arguments @($entry.args) -WorkingDirectory $resolvedRoot -Server $Server -ExpectedTools $ExpectedTools -Environment $environment -ClientName "registered-mcp-smoke"
  Write-Host (@{ plugin = $manifest.name; server = $Server; platform = $platform; command = $command; args = @($entry.args); initialize = "ok"; tools = @($result.Tools).Count } | ConvertTo-Json -Compress -Depth 20)
}

Invoke-McpRegistration "plugins/codexy" watcher 5
Invoke-McpRegistration "plugins/codexy-devtools" lsp 8
Invoke-McpRegistration "plugins/codexy-devtools" codegraph 6
