param(
    [ValidateSet("--check", "--fix")][string]$Mode,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Path
)

$ErrorActionPreference = "Stop"
if ($env:CODEXY_PSSCRIPTANALYZER_PATH) {
    Import-Module (Join-Path $env:CODEXY_PSSCRIPTANALYZER_PATH "PSScriptAnalyzer.psd1") -Force
} else {
    Import-Module PSScriptAnalyzer -Force
}

$findings = foreach ($file in $Path) {
    if ($Mode -eq "--fix") {
        $formatted = Invoke-Formatter -ScriptDefinition (Get-Content -Raw -LiteralPath $file)
        [IO.File]::WriteAllText($file, $formatted, [Text.UTF8Encoding]::new($false))
    }
    Invoke-ScriptAnalyzer -Path $file -Severity ParseError, Error, Warning
}
if ($findings) {
    $findings | Format-Table -AutoSize | Out-String | Write-Error
}
