$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "mcp-smoke-support.ps1")
$temporaryRoot = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) { [System.IO.Path]::GetTempPath() } else { $env:RUNNER_TEMP }
$cacheRoot = Join-Path $temporaryRoot "codexy-mcp-registration-cache"
New-Item -ItemType Directory -Force -Path $cacheRoot | Out-Null
# When a selected wheel is supplied, force registration to resolve from prepared local artifacts only.
$selectedWheelDir = $env:CODEXY_SELECTED_MCP_WHEEL_DIR
if (-not [string]::IsNullOrWhiteSpace($selectedWheelDir)) {
  $env:UV_NO_INDEX = "1"
  $env:UV_FIND_LINKS = $selectedWheelDir
  $env:UV_CACHE_DIR = Join-Path $temporaryRoot "codexy-registered-uv-cache"
}

# Resolve the host platform once so each manifest's advertised support is checked against the actual runner.
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
  $hasMcpDeclaration = $null -ne $manifest.PSObject.Properties["mcpServers"] -or $null -ne $manifest.PSObject.Properties["mcp_servers"]
  if ($PluginRoot -eq "plugins/codexy" -and -not $hasMcpDeclaration) {
    $capabilityPath = Join-Path $resolvedRoot ".codex-plugin/mcp-capability.json"
    if (-not (Test-Path -LiteralPath $capabilityPath -PathType Leaf)) { throw "core package is missing its explicit no-MCP capability contract: $capabilityPath" }
    $capability = Get-Content -Raw -LiteralPath $capabilityPath | ConvertFrom-Json
    $propertyNames = @($capability.PSObject.Properties.Name | Sort-Object)
    if ($propertyNames.Count -ne 3 -or [string]::Join(",", $propertyNames) -cne "component,schema,servers" -or $capability.schema -cne "codexy.mcp-capability.v1" -or $capability.component -cne "core" -or @($capability.servers).Count -ne 0) {
      throw "core package has an invalid no-MCP capability contract: $capabilityPath"
    }
    $configPath = Join-Path $resolvedRoot ".mcp.json"
    if (Test-Path -LiteralPath $configPath) { throw "core package has residual MCP configuration without a manifest declaration: $configPath" }
    # A core package without MCP metadata must be checked for absence, not started as Watcher.
    Write-Output (@{
        plugin = $manifest.name
        server = $Server
        status = "not-registered"
        reason = "core manifest and configuration omit Watcher MCP"
      } | ConvertTo-Json -Compress)
    return
  }
  if ($PluginRoot -eq "plugins/codexy" -and (Test-Path -LiteralPath (Join-Path $resolvedRoot ".codex-plugin/mcp-capability.json"))) {
    throw "core package cannot declare both a no-MCP capability and MCP registration"
  }
  if (@($manifest.supportedPlatforms) -notcontains $platform) {
    Write-Output (@{
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
  $runtimePrefix = if ($PluginRoot -eq "plugins/codexy") { "CORE" } elseif ($PluginRoot -eq "plugins/codexy-devtools") { "DEVTOOLS" } else { throw "unsupported registered MCP plugin root: $PluginRoot" }
  $runtimeDir = [Environment]::GetEnvironmentVariable("CODEXY_RUNTIME_${runtimePrefix}_DIR")
  if (-not [string]::IsNullOrWhiteSpace($runtimeDir)) {
    if (-not [System.IO.Path]::IsPathRooted($runtimeDir) -or -not (Test-Path -LiteralPath $runtimeDir -PathType Container)) { throw "registered MCP runtime directory is not an absolute directory: $runtimeDir" }
    $environment["CODEXY_RUNTIME_DIR"] = $runtimeDir
  }
  $result = Invoke-McpProtocol -FileName $command -Arguments @($entry.args) -WorkingDirectory $resolvedRoot -Server $Server -ExpectedTools $ExpectedTools -Environment $environment -ClientName "registered-mcp-smoke"
  Write-Output (@{ plugin = $manifest.name; server = $Server; platform = $platform; command = $command; args = @($entry.args); initialize = "ok"; tools = @($result.Tools).Count } | ConvertTo-Json -Compress -Depth 20)
}

Invoke-McpRegistration "plugins/codexy" watcher 5
Invoke-McpRegistration "plugins/codexy-devtools" lsp 8
Invoke-McpRegistration "plugins/codexy-devtools" codegraph 8
