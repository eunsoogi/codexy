$ErrorActionPreference = "Stop"

$condition = $env:CODEXY_MEASUREMENT_CONDITION
if ($condition -notin @("cold", "warm")) {
    throw "unsupported measurement condition: $condition"
}

$config = Join-Path $PSScriptRoot "..\packages\codexy-runtime\rust-toolchain.toml"
$configText = Get-Content -LiteralPath $config -Raw
$toolchain = [regex]::Match($configText, '(?m)^channel = "([^"]+)"$').Groups[1].Value
$toolchainProfile = [regex]::Match($configText, '(?m)^profile = "([^"]+)"$').Groups[1].Value
if ([string]::IsNullOrWhiteSpace($toolchain) -or [string]::IsNullOrWhiteSpace($toolchainProfile)) {
    throw "rust-toolchain.toml must define channel and profile"
}

$components = @([regex]::Match($configText, '(?m)^components = \[([^\]]*)\]$').Groups[1].Value -split ',' |
    ForEach-Object { $_.Trim().Trim('"') } | Where-Object { $_ })
$rustupArgs = @("toolchain", "install", $toolchain, "--profile", $toolchainProfile)
foreach ($component in $components) {
    $rustupArgs += @("--component", $component)
}

if ($condition -eq "cold") {
    & rustup @rustupArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

& rustup default $toolchain
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$measurementFile = Join-Path $env:RUNNER_TEMP "codexy-rust-measurement\metrics\measurement.txt"
if (-not (Test-Path -LiteralPath $measurementFile -PathType Leaf)) {
    throw "measurement marker is missing"
}
$rustcVersion = (& rustc --version).Trim()
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$cargoVersion = (& cargo --version).Trim()
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$rustHost = ((& rustc -vV | Select-String '^host: ').Line -replace '^host: ', '').Trim()
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
@("toolchain=$toolchain", "rustc=$rustcVersion", "cargo=$cargoVersion", "host=$rustHost") |
    Add-Content -LiteralPath $measurementFile
