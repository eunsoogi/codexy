$ErrorActionPreference = "Stop"

$mode = [string]$env:CACHE_MODE
$headSha = [string]$env:HEAD_SHA
$condition = [string]$env:CONDITION
$repeatId = [string]$env:REPEAT_ID
$profiling = if ($null -eq $env:PROFILE_ENABLED) { "false" } else { [string]$env:PROFILE_ENABLED }
$cacheIdentity = [string]$env:CACHE_IDENTITY
$shardIndex = [string]$env:SHARD_INDEX

if ($mode -notin @("isolated", "normal")) { throw "unsupported measurement cache mode: $mode" }
if ($headSha -notmatch '^[0-9a-f]{40}$' -or (git rev-parse HEAD).Trim() -ne $headSha) {
    throw "checked out source commit does not match workflow input"
}
if ($condition -notmatch '^(cold|warm)$') { throw "unsupported measurement condition" }
if ($repeatId -notmatch '^[A-Za-z0-9._-]{1,64}$' -or $cacheIdentity -notmatch '^[A-Za-z0-9._-]{1,64}$') {
    throw "measurement identities must use 1-64 letters, digits, dot, underscore, or hyphen"
}
if ($profiling -notmatch '^(true|false)$') { throw "unsupported profiling flag" }
if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP) -or [string]::IsNullOrWhiteSpace($env:GITHUB_ENV)) {
    throw "runner temp and GITHUB_ENV are required"
}

$root = Join-Path $env:RUNNER_TEMP "codexy-rust-measurement"
Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path (Join-Path $root "metrics") | Out-Null

$metadata = @(
    "head_sha=$headSha"
    "condition=$condition"
    "cache_mode=$mode"
    "repeat_id=$repeatId"
    "profiling=$profiling"
    "cache_identity=$cacheIdentity"
    "shard_index=$shardIndex"
    "runner_os=$env:RUNNER_OS"
)
if ($condition -eq "cold") { $metadata += "cache_state=cold-empty" }
Set-Content -LiteralPath (Join-Path $root "metrics\measurement.txt") -Value $metadata

if ($mode -eq "isolated") {
    New-Item -ItemType Directory -Force -Path (Join-Path $root "cargo"), (Join-Path $root "rustup"), (Join-Path $root "target") | Out-Null
    Add-Content -LiteralPath $env:GITHUB_ENV -Value @(
        "CARGO_HOME=$(Join-Path $root 'cargo')"
        "RUSTUP_HOME=$(Join-Path $root 'rustup')"
        "CARGO_TARGET_DIR=$(Join-Path $root 'target')"
    )
}

if ($profiling -eq "true") {
    Add-Content -LiteralPath $env:GITHUB_ENV -Value @(
        "CODEXY_PROFILE_METRICS=$(Join-Path $root 'metrics\profile.metrics')"
        "CODEXY_PROFILE_COMMAND_METRICS_DIR=$(Join-Path $root 'metrics\commands')"
    )
}
