param(
    [string]$Suite,
    [string]$ExpectedHead,
    [string]$ExpectedIdentity
)
$ErrorActionPreference = "Stop"

function Get-CompilerIdentity {
    $metadata = @(& rustc -vV)
    if ($LASTEXITCODE -ne 0) { throw "rustc identity query failed" }
    $text = $metadata -join "`n"
    $release = [regex]::Match($text, '(?m)^release:\s+(\S+)$').Groups[1].Value
    $targetHost = [regex]::Match($text, '(?m)^host:\s+(\S+)$').Groups[1].Value
    if (-not $release -or -not $targetHost) { throw "rustc must report release and host" }
    # The suffix excludes caches written before preparation fixed their identity.
    return "$release-$targetHost-prepared-v1"
}

if ($ExpectedIdentity) {
    $actual = Get-CompilerIdentity
    if ($actual -ne $ExpectedIdentity) { throw "restored compiler $actual does not match cache identity $ExpectedIdentity" }
    Write-Output "normal-cache-compiler-verified identity=$actual"
    & "$PSScriptRoot/report-rust-cache-docs.ps1" $Suite -Phase prepared
    return
}
if (-not $ExpectedHead -or (git rev-parse HEAD).Trim() -ne $ExpectedHead) {
    throw "checked out source commit does not match requested head"
}
& "$PSScriptRoot/report-rust-cache-docs.ps1" $Suite -Phase runner
& "$PSScriptRoot/../../scripts/ensure-rust-toolchain.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$identity = Get-CompilerIdentity
Write-Output "normal-cache-compiler-prepared identity=$identity"
"identity=$identity" >> $env:GITHUB_OUTPUT
