param(
    [ValidateSet("--check", "--fix")][string]$Mode,
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$ModulePath,
    [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)][string[]]$Path
)

$ErrorActionPreference = "Stop"
$repository = (Resolve-Path -LiteralPath (Split-Path -Parent $PSScriptRoot)).Path
$files = @($Path | ForEach-Object {
    $candidate = Join-Path $repository $_
    $relative = [IO.Path]::GetRelativePath($repository, $candidate)
    if ([IO.Path]::IsPathRooted($_) -or $relative -eq ".." -or $relative.StartsWith("..$([IO.Path]::DirectorySeparatorChar)")) {
        throw "lint input must remain in the repository: $_"
    }
    $cursor = $repository
    foreach ($part in $relative -split '[\\/]') {
        $cursor = Join-Path $cursor $part
        if ((Get-Item -LiteralPath $cursor -Force).LinkType) {
            throw "lint input must not cross a link: $_"
        }
    }
    $item = Get-Item -LiteralPath $candidate -Force
    if ($item.LinkType -or -not ($item -is [System.IO.FileInfo])) {
        throw "lint input must be a regular file: $_"
    }
    $item.FullName
})

if ($ModulePath) {
    Import-Module (Join-Path $ModulePath "PSScriptAnalyzer.psd1") -RequiredVersion $Version -ErrorAction Stop
} else {
    Import-Module PSScriptAnalyzer -RequiredVersion $Version -ErrorAction Stop
}
if ($Mode -eq "--fix") {
    foreach ($file in $files) {
        $formatted = Invoke-Formatter -ScriptDefinition (Get-Content -Raw -LiteralPath $file)
        [IO.File]::WriteAllText($file, $formatted, [Text.UTF8Encoding]::new($false))
    }
}

$findings = @(Invoke-ScriptAnalyzer -Path $files -Recurse -Severity Error, Warning)
if ($findings.Count -gt 0) {
    $findings | Format-Table -AutoSize | Out-String | Write-Error
    exit 1
}
