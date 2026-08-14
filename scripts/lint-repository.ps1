$ErrorActionPreference = "Stop"
$script = Join-Path $PSScriptRoot "lint-repository.py"
& python $script @args
exit $LASTEXITCODE
