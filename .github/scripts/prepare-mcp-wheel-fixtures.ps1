$ErrorActionPreference = "Stop"

python -m pip install --disable-pip-version-check build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
python -m build --wheel --outdir dist packages/getcodexy
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$selectedVersion = [string](Get-Content -Raw -LiteralPath "plugins/codexy/.codex-plugin/plugin.json" | ConvertFrom-Json).version
$candidateVersion = python -c "import tomllib; print(next(package['version'] for package in tomllib.load(open('packages/getcodexy/uv.lock', 'rb'))['package'] if package['name'] == 'getcodexy'))"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$projectVersion = python -c "import tomllib; print(tomllib.load(open('packages/getcodexy/pyproject.toml', 'rb'))['project']['version'])"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($candidateVersion -ne $projectVersion) { throw "candidate package version projections are not synchronized" }

$selectedWheelDir = (Resolve-Path -LiteralPath "dist").Path
if ($candidateVersion -ne $selectedVersion) {
  $selectedSource = Join-Path $env:RUNNER_TEMP "codexy-selected-version-wheel-source"
  $selectedWheelDir = Join-Path $env:RUNNER_TEMP "codexy-selected-version-wheel"
  if (Test-Path -LiteralPath $selectedSource) { Remove-Item -LiteralPath $selectedSource -Recurse -Force }
  if (Test-Path -LiteralPath $selectedWheelDir) { Remove-Item -LiteralPath $selectedWheelDir -Recurse -Force }
  Copy-Item -LiteralPath "packages/getcodexy" -Destination $selectedSource -Recurse
  $projectPath = Join-Path $selectedSource "pyproject.toml"
  $projectText = Get-Content -Raw -LiteralPath $projectPath
  $projectRewritten = [regex]::Replace($projectText, '(?m)^version = "[^"]+"$', ('version = "' + $selectedVersion + '"'), 1)
  if ($projectRewritten -eq $projectText) { throw "selected-version fixture did not rewrite pyproject version" }
  [System.IO.File]::WriteAllText($projectPath, $projectRewritten, [System.Text.UTF8Encoding]::new($false))
  $lockPath = Join-Path $selectedSource "uv.lock"
  $lockText = Get-Content -Raw -LiteralPath $lockPath
  $lockPattern = '(?ms)(\[\[package\]\]\r?\nname = "getcodexy"\r?\nversion = ")[^"]+(")'
  $lockRewritten = [regex]::Replace($lockText, $lockPattern, ('${1}' + $selectedVersion + '${2}'), 1)
  if ($lockRewritten -eq $lockText) { throw "selected-version fixture did not rewrite uv.lock version" }
  [System.IO.File]::WriteAllText($lockPath, $lockRewritten, [System.Text.UTF8Encoding]::new($false))
  python -m build --wheel --outdir $selectedWheelDir $selectedSource
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$venv = Join-Path (Get-Location).Path ".candidate-venv"
python -m venv $venv
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$venvPython = Join-Path $venv $(if ($IsWindows) { "Scripts/python.exe" } else { "bin/python" })
$venvRuntime = Join-Path $venv $(if ($IsWindows) { "Scripts/codexy-mcp-runtime.exe" } else { "bin/codexy-mcp-runtime" })
& $venvPython -m pip install --no-index --find-links dist "getcodexy==$candidateVersion"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$candidateRuntime = (Resolve-Path -LiteralPath $venvRuntime).Path
"CODEXY_SELECTED_MCP_WHEEL_DIR=$selectedWheelDir" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
"GETCODEXY_CANDIDATE_RUNTIME=$candidateRuntime" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append

if (Test-Path -LiteralPath ".agents/plugins/runtime-activation.json" -PathType Leaf) {
  bash scripts/download-selected-runtime-package.sh dist/selected-runtime-package.tar.gz
  if ($LASTEXITCODE -ne 0) { throw "selected runtime download failed" }
  $archive = (Resolve-Path -LiteralPath "dist/selected-runtime-package.tar.gz").Path
  $contract = Get-Content -Raw -LiteralPath ".agents/plugins/release-publish-contract.json" | ConvertFrom-Json
  $release = Get-Content -Raw -LiteralPath "plugins/codexy-devtools/runtime-release.json" | ConvertFrom-Json
  $record = Get-Content -Raw -LiteralPath ".agents/plugins/runtime-activation.json" | ConvertFrom-Json
  $archiveVersion = [string](python -c 'import json,sys,tarfile; archive=tarfile.open(sys.argv[1]); names=[name for name in archive.getnames() if name.startswith("plugins/") and name.endswith("/.codex-plugin/plugin.json")]; assert len(names) == 1; print(json.load(archive.extractfile(names[0]))["version"])' $archive)
  $observedDigest = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
  $expectedSelectedVersion = [string]$contract.bootstrap.selectedVersion
  $expectedCandidateVersion = [string]$contract.bootstrap.candidateVersion
  $activatedVersion = [string]$release.artifact.tag.TrimStart("v")
  if ($selectedVersion -ne $expectedSelectedVersion) { throw "selected wheel version differs from contract" }
  if ($candidateVersion -ne $expectedCandidateVersion) { throw "candidate wheel version differs from contract" }
  if ($expectedSelectedVersion -ne $activatedVersion) { throw "selected and activated runtime versions differ" }
  if ($record.candidate.source.commit -ne $release.source.commit) { throw "selected and activated runtime sources differ" }
  if ($archiveVersion -ne $activatedVersion) { throw "selected runtime archive version differs" }
  if (Test-Path -LiteralPath "dist/public-release" -PathType Leaf) {
    $expectedDigest = [string](Get-Content -Raw -LiteralPath "public-release/runtime-release-receipt.json" | ConvertFrom-Json).artifact.sha256
  } elseif (Test-Path -LiteralPath "dist/legacy-public" -PathType Leaf) {
    $expectedDigest = [string]$release.artifact.sha256
  } else {
    $expectedDigest = [string]$record.artifact.sha256
  }
  if ($observedDigest -ne $expectedDigest) { throw "selected runtime archive digest differs" }
  $runtimeRoot = Join-Path $env:RUNNER_TEMP "codexy-selected-runtime"
  if (Test-Path -LiteralPath $runtimeRoot) { throw "selected runtime extraction root must be fresh" }
  New-Item -ItemType Directory -Path $runtimeRoot | Out-Null
  python -c 'import sys; from pathlib import Path; sys.path.insert(0, "packages/getcodexy/src"); from codexy_runtime_tools.package_archive import _safe_extract_tar; _safe_extract_tar(Path(sys.argv[1]), Path(sys.argv[2]))' $archive $runtimeRoot
  if ($LASTEXITCODE -ne 0) { throw "selected runtime extraction failed" }
  $runtimeDir = Join-Path $runtimeRoot "plugins/codexy-devtools/runtime"
  if (-not (Test-Path -LiteralPath $runtimeDir -PathType Container)) { throw "selected runtime directory is missing" }
  $stagedArchive = -not (Test-Path -LiteralPath "dist/public-release" -PathType Leaf) -and -not (Test-Path -LiteralPath "dist/legacy-public" -PathType Leaf)
  if ($stagedArchive) {
    foreach ($className in @("coreWatcherMcp", "devtoolsMcp")) {
      $class = $record.candidate.classes.PSObject.Properties[$className].Value
      foreach ($platformEntry in $class.platforms.PSObject.Properties) {
        $serverEntries = if ($className -eq "coreWatcherMcp") { @($platformEntry.Value) } else { @($platformEntry.Value.PSObject.Properties | ForEach-Object { $_.Value }) }
        foreach ($serverEntry in $serverEntries) {
          $runtimePath = Join-Path $runtimeRoot ("plugins/codexy-devtools/" + [string]$serverEntry.path)
          if (-not (Test-Path -LiteralPath $runtimePath -PathType Leaf)) { throw "selected runtime binary is missing: $($serverEntry.path)" }
          $binaryDigest = (Get-FileHash -Algorithm SHA256 -LiteralPath $runtimePath).Hash.ToLowerInvariant()
          if ($binaryDigest -ne [string]$serverEntry.sha256) { throw "selected runtime binary digest differs: $($serverEntry.path)" }
        }
      }
    }
  }
  "CODEXY_RUNTIME_CORE_DIR=$runtimeDir" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "CODEXY_RUNTIME_DEVTOOLS_DIR=$runtimeDir" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  Write-Host (@{ selected_version = $selectedVersion; candidate_version = $candidateVersion; activated_version = $activatedVersion; runtime_source_commit = $release.source.commit; runtime_archive_version = $archiveVersion; runtime_archive_sha256 = $expectedDigest; runtime_directory = $runtimeDir; staged_archive = $stagedArchive } | ConvertTo-Json -Compress)
} else {
  Write-Host (@{ candidate_version = $candidateVersion; selected_version = $selectedVersion; selected_wheel_dir = $selectedWheelDir; candidate_runtime = $candidateRuntime } | ConvertTo-Json -Compress)
}
