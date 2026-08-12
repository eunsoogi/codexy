param([ValidateSet('issue', 'pr')][string]$rule)
$ErrorActionPreference = 'Stop'
$payload = [Console]::In.ReadToEnd() | ConvertFrom-Json
$title = $payload.tool_input.title
if ($null -eq $title) { exit 0 }
$conventional = '^[a-z0-9-]+(?:\([a-z0-9_/-]+\))?!?:\s+\S'
$invalid = ($rule -eq 'issue' -and ($title.Length -eq 0 -or -not [char]::IsUpper($title[0]) -or $title -match $conventional)) -or ($rule -eq 'pr' -and $title -notmatch $conventional)
if ($invalid) {
  $reason = if ($rule -eq 'issue') { 'issue title must be uppercase descriptive prose, not Conventional Commit syntax' } else { 'PR title must use Conventional Commit syntax' }
  [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"CODEXY_GITHUB_ADMISSION: ' + $reason + '"}}')
}
