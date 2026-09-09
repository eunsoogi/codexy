$ErrorActionPreference = "Stop"

$archive = Join-Path (Get-Location).Path "codexy-windows-rust-prepared-download/codexy-windows-rust-prepared.tar"
if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) { throw "shared Rust dependency archive is missing" }
New-Item -ItemType Directory -Force -Path (Join-Path $HOME ".cargo"), "packages/codexy-runtime" | Out-Null
& tar.exe -xf $archive -C $HOME ".cargo/registry"
if ($LASTEXITCODE -ne 0) { throw "shared Cargo registry archive extraction failed" }
& tar.exe -xf $archive -C (Get-Location).Path "packages/codexy-runtime/target"
if ($LASTEXITCODE -ne 0) { throw "shared Cargo target archive extraction failed" }
if (-not (Test-Path -LiteralPath (Join-Path $HOME ".cargo/registry") -PathType Container)) { throw "shared Cargo registry extraction is missing" }
if (-not (Test-Path -LiteralPath "packages/codexy-runtime/target" -PathType Container)) { throw "shared Cargo target extraction is missing" }
