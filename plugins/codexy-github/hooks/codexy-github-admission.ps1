param([ValidateSet('issue', 'pr')][string]$rule)
$ErrorActionPreference = 'Stop'

function Test-InvalidCharacter([string]$value) {
  foreach ($character in $value.ToCharArray()) {
    $code = [int][char]$character
    if ($code -lt 32 -or ($code -ge 128 -and $code -le 159) -or $code -eq 127 -or $code -in @(8232, 8233)) { return $true }
  }
  return $false
}

function Test-TerminalReference([string]$value) {
  $candidate = $value.Trim()
  while ($candidate.EndsWith('.') -or $candidate.EndsWith(',')) {
    $candidate = $candidate.Substring(0, $candidate.Length - 1).TrimEnd()
  }
  return $candidate -match '(?:^|\s)(?:#[0-9]+|\(#[0-9]+\)|\[#[0-9]+\]|\((?:pr|issue)\s+#[0-9]+\)|(?:pr|issue)\s+#[0-9]+)$'
}

function Test-ConventionalPrefix([string]$value) {
  $prefix = $value -replace '!$'
  return $prefix -cmatch '^[a-z0-9-]+\([a-z0-9_/-]+\)$'
}

function Test-LabelSeparator([string]$value) {
  return $value -match '^(?:[:：]|[-–—](?:$|[ \t]))'
}

function Test-PrTitle([string]$value) {
  if ([string]::IsNullOrEmpty($value) -or (Test-InvalidCharacter $value) -or $value -notmatch ': ') { return $false }
  $parts = $value -split ': ', 2
  return ($parts[1].Trim().Length -gt 0 -and (Test-ConventionalPrefix $parts[0]) -and -not (Test-TerminalReference $parts[1]))
}

function Test-Category([string]$value) {
  if ($value -match '^\[[A-Za-z0-9-]+(?:\([A-Za-z0-9_/-]+\))?!?\]') { return $true }
  $prefix = '[A-Za-z0-9-]+[ \t]*(?:\([ \t]*[A-Za-z0-9_/-]+[ \t]*\))?[ \t]*!?'
  if ($value -match "^$prefix[ \t]*[:：]") { return $true }
  if ($value -match "^$prefix[ \t]*[-–—][ \t]") { return $true }
  $scoped = [regex]::Match($value, '^[A-Za-z0-9-]+[ \t]*\([ \t]*[A-Za-z0-9_/-]+[ \t]*\)[ \t]*!?')
  if ($scoped.Success) {
    $prefixValue = $scoped.Value
    while ($prefixValue.EndsWith(' ') -or $prefixValue.EndsWith("`t")) { $prefixValue = $prefixValue.Substring(0, $prefixValue.Length - 1) }
    $rest = $value.Substring($prefixValue.Length)
    if ($rest.Length -eq 0 -or $rest.StartsWith(' ') -or $rest.StartsWith("`t") -or (Test-LabelSeparator $rest)) { return $true }
  }
  $banged = [regex]::Match($value, '^[A-Za-z0-9-]+[ \t]*!')
  if ($banged.Success) {
    $rest = $value.Substring($banged.Value.Length)
    if ($rest.Length -eq 0 -or $rest.StartsWith(' ') -or $rest.StartsWith("`t") -or (Test-LabelSeparator $rest)) { return $true }
  }
  $trimmed = $value.TrimEnd(' ', "`t")
  return $trimmed -match '^[A-Za-z0-9-]+!?$'
}

function Test-IssueTitle([string]$value) {
  if ([string]::IsNullOrEmpty($value) -or (Test-InvalidCharacter $value) -or [int][char]$value[0] -lt 65 -or [int][char]$value[0] -gt 90) { return $false }
  return -not (Test-Category $value)
}

$payload = [Console]::In.ReadToEnd() | ConvertFrom-Json
$inputObject = $payload.tool_input
$title = $inputObject.title
if ($null -eq $title) { exit 0 }
$tool = $payload.tool_name
if ($tool -is [string] -and $tool.StartsWith('mcp__codex_apps__github_') -and ($null -ne $inputObject.repository_full_name -or $null -ne $inputObject.repo_full_name)) { exit 0 }
$valid = if ($rule -eq 'issue') { Test-IssueTitle $title } else { Test-PrTitle $title }
if (-not $valid) {
  $reason = if ($rule -eq 'issue') { 'issue title must be uppercase descriptive prose, not Conventional Commit syntax' } else { 'PR title must use Conventional Commit syntax' }
  [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_GITHUB_ADMISSION: ' + $reason + '"}}')
}
