$ErrorActionPreference = "Stop"

$bundleRoot = Join-Path $env:RUNNER_TEMP "codexy-public-marketplace-bundle"
$verified = python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$release = $verified | ConvertFrom-Json
if (-not $release.extracted_root) { throw "verified public bundle has no extracted root" }
Write-Host (
    @{
        prior_public_release = $release.release_tag
        baseline_source = $release.baseline_source
        baseline_commit = $release.baseline_commit
        bundle_sha256 = $release.bundle_sha256
        legacy_watcher_present = [bool]$release.legacy_watcher_binary
    } | ConvertTo-Json -Compress
)

# Keep the verified historical package available only to lifecycle upgrade fixtures that need its Watcher.
$legacyRelease = $release
if (-not $legacyRelease.legacy_watcher_binary) {
    $legacyRoot = Join-Path $env:RUNNER_TEMP "codexy-legacy-upgrade-bundle"
    $legacyVerified = python scripts/verify_public_marketplace_bundle.py --legacy-upgrade --output-dir $legacyRoot
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $legacyRelease = $legacyVerified | ConvertFrom-Json
}
if ($legacyRelease.legacy_upgrade_fixture -ne $true -or -not $legacyRelease.legacy_watcher_binary) {
    throw "verified legacy upgrade fixture is missing its expected Watcher binary"
}

$env:CODEXY_PUBLIC_BUNDLE_ROOT = [string]$release.extracted_root
$env:CODEXY_TEST_LEGACY_MARKETPLACE_ROOT = [string]$legacyRelease.extracted_root
"CODEXY_PUBLIC_BUNDLE_ROOT=$($release.extracted_root)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
"CODEXY_TEST_LEGACY_MARKETPLACE_ROOT=$($legacyRelease.extracted_root)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
Write-Host (
    @{
        legacy_upgrade_release = $legacyRelease.release_tag
        legacy_baseline_source = $legacyRelease.baseline_source
        legacy_bundle_sha256 = $legacyRelease.bundle_sha256
    } | ConvertTo-Json -Compress
)
