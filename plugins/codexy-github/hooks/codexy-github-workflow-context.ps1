$ErrorActionPreference = 'Stop'
$payload = [Console]::In.ReadToEnd()
if ($payload -match '(?i)github|issue|pull[ -]?request|review|merge') {
  [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy GitHub workflow is installed. Use $git-workflow; its package-owned generic admission hooks are active."}}')
}
