$ErrorActionPreference = "Stop"

if (-not (Get-Command rg -ErrorAction SilentlyContinue)) {
    choco install ripgrep --yes --no-progress
}

if (-not (Get-Command rg -ErrorAction SilentlyContinue)) {
    throw "Windows Rust tests require the ripgrep (rg) archive scanner"
}
