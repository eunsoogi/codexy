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
Write-Host (@{ candidate_version = $candidateVersion; selected_version = $selectedVersion; selected_wheel_dir = $selectedWheelDir; candidate_runtime = $candidateRuntime } | ConvertTo-Json -Compress)
