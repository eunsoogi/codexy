$ErrorActionPreference = "Stop"

$mode = [string]$env:CODEXY_MEASUREMENT_MODE
$condition = [string]$env:CODEXY_MEASUREMENT_CONDITION
$cacheHit = if ($null -eq $env:CODEXY_MEASUREMENT_CACHE_HIT) { "" } else { [string]$env:CODEXY_MEASUREMENT_CACHE_HIT }

function Add-IsolatedCacheState {
    param([string]$State)

    $root = [string]$env:CODEXY_MEASUREMENT_ROOT
    if ([string]::IsNullOrWhiteSpace($root)) {
        throw "measurement root is missing"
    }
    $measurementFile = Join-Path $root "metrics\measurement.txt"
    if (-not (Test-Path -LiteralPath $measurementFile -PathType Leaf)) {
        throw "measurement marker is missing"
    }
    Add-Content -LiteralPath $measurementFile "cache_state=$State"
}

switch ("${mode}:${condition}") {
    "normal:cold" {
        if ($cacheHit -eq "true") {
            throw "normal cold measurement requires an exact cache miss (cache-hit=true)"
        }
        if ($cacheHit -notin @("", "false")) {
            throw "normal cold measurement received an unexpected cache-hit value: $cacheHit"
        }
        $displayHit = if ([string]::IsNullOrEmpty($cacheHit)) { "empty" } else { $cacheHit }
        Write-Output "normal cold measurement confirmed cache miss (cache-hit=$displayHit)"
    }
    "normal:warm" {
        if ($cacheHit -ne "true") {
            $displayHit = if ([string]::IsNullOrEmpty($cacheHit)) { "empty" } else { $cacheHit }
            throw "normal warm measurement requires an exact cache hit (cache-hit=$displayHit)"
        }
        Write-Output "normal warm measurement confirmed exact cache hit"
    }
    "isolated:cold" {
    }
    "isolated:warm" {
        if ($cacheHit -eq "true") {
            Add-IsolatedCacheState "warm-hit"
        } else {
            Add-IsolatedCacheState "warm-miss"
            throw "isolated warm measurement requires an exact cache hit"
        }
    }
    default {
        throw "unsupported measurement cache mode or condition: ${mode}:${condition}"
    }
}
