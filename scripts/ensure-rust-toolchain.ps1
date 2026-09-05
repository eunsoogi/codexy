$ErrorActionPreference = "Stop"

$config = Join-Path $PSScriptRoot "..\packages\codexy-runtime\rust-toolchain.toml"
$configText = Get-Content -LiteralPath $config -Raw
$toolchain = [regex]::Match($configText, '(?m)^channel = "([^"]+)"$').Groups[1].Value
$profile = [regex]::Match($configText, '(?m)^profile = "([^"]+)"$').Groups[1].Value
$components = @([regex]::Match($configText, '(?m)^components = \[([^\]]*)\]$').Groups[1].Value -split ',' |
    ForEach-Object { $_.Trim().Trim('"') } | Where-Object { $_ })

if ([string]::IsNullOrWhiteSpace($toolchain) -or [string]::IsNullOrWhiteSpace($profile)) {
    throw "rust-toolchain.toml must define channel and profile"
}

function Get-RustupState {
    $output = @(& rustup show 2>$null)
    if ($LASTEXITCODE -ne 0) {
        return $null
    }
    $hostLine = $output | Where-Object { $_ -match '^Default host:\s+(\S+)$' } | Select-Object -First 1
    $activeLine = $output | Where-Object { $_ -match '^name:\s+(\S+)$' } | Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($hostLine) -or [string]::IsNullOrWhiteSpace($activeLine)) {
        return $null
    }
    [pscustomobject]@{
        Host = [regex]::Match($hostLine, '^Default host:\s+(\S+)$').Groups[1].Value
        Active = [regex]::Match($activeLine, '^name:\s+(\S+)$').Groups[1].Value
    }
}

function Test-ExpectedToolchain {
    param($State)

    if ($null -eq $State) {
        return $false
    }
    $expected = "{0}-{1}" -f $toolchain, $State.Host
    return $State.Active -eq $expected
}

function Test-RequiredComponents {
    param([string]$Active)

    $installed = @(& rustup component list --toolchain $Active 2>$null)
    if ($LASTEXITCODE -ne 0) {
        return $false
    }
    foreach ($component in $components) {
        $pattern = "^{0}(?:-|\s).*\(installed\)$" -f [regex]::Escape($component)
        if (-not ($installed | Where-Object { $_ -match $pattern })) {
            return $false
        }
    }
    return $true
}

function Install-ConfiguredToolchain {
    $arguments = @("toolchain", "install", $toolchain, "--profile", $profile)
    foreach ($component in $components) {
        $arguments += @("--component", $component)
    }
    & rustup @arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}

$state = Get-RustupState
if ($null -ne $state -and -not (Test-ExpectedToolchain $state)) {
    throw "root active Rust toolchain $($state.Active) does not match configured $toolchain-$($state.Host)"
}

$installed = $false
if ($null -eq $state -or -not (Test-RequiredComponents $state.Active)) {
    Install-ConfiguredToolchain
    $installed = $true
    $state = Get-RustupState
}

if ($null -eq $state -or -not (Test-ExpectedToolchain $state) -or
    -not (Test-RequiredComponents $state.Active)) {
    throw "configured Rust toolchain is not the root active toolchain with required components"
}

if ($installed) {
    Write-Output "installed configured Rust toolchain $toolchain"
} else {
    Write-Output "configured Rust toolchain $toolchain is already active with required components"
}
