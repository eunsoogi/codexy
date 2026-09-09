$ErrorActionPreference = "Stop"

$manifest = "packages/codexy-runtime/Cargo.toml"
& cargo test --manifest-path $manifest --locked --no-run --lib --bins
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$registry = Join-Path $HOME ".cargo/registry"
$target = "packages/codexy-runtime/target"
if (-not (Test-Path -LiteralPath $registry -PathType Container)) { throw "Cargo registry was not prepared" }
if (-not (Test-Path -LiteralPath $target -PathType Container)) { throw "Cargo target was not prepared" }

$archive = Join-Path (Get-Location).Path "codexy-windows-rust-prepared.tar"
if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
& tar.exe -cf $archive -C $HOME ".cargo/registry" -C (Get-Location).Path $target
if ($LASTEXITCODE -ne 0) { throw "shared Rust dependency archive failed" }
$entries = @(& tar.exe -tf $archive)
if ($LASTEXITCODE -ne 0 -or -not ($entries -contains ".cargo/registry/") -or -not ($entries -contains "$target/")) {
    throw "shared Rust dependency archive is missing a required root"
}
