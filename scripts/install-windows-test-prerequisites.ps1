$ErrorActionPreference = "Stop"

$scanner = Get-Command rg -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $scanner) {
    $scanner = Get-Command grep -CommandType Application -ErrorAction SilentlyContinue
}

if ($null -eq $scanner) {
    $searchDirectories = @()
    if ($env:ProgramFiles) {
        $searchDirectories += Join-Path $env:ProgramFiles "Git\usr\bin"
    }
    if ($env:SystemDrive) {
        $searchDirectories += Join-Path $env:SystemDrive "msys64\usr\bin"
    }
    foreach ($directory in $searchDirectories) {
        $candidate = Join-Path $directory "grep.exe"
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            continue
        }
        $env:Path = "$directory$([IO.Path]::PathSeparator)$env:Path"
        if ($env:GITHUB_PATH) {
            Add-Content -LiteralPath $env:GITHUB_PATH -Value $directory
        }
        $scanner = Get-Command grep -CommandType Application -ErrorAction SilentlyContinue
        if ($null -ne $scanner) {
            break
        }
    }
}

if ($null -eq $scanner) {
    throw "Windows Rust tests require the ripgrep (rg) or grep archive scanner"
}
