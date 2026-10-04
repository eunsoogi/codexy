$ErrorActionPreference = 'Stop'
$payload = [Console]::In.ReadToEnd()
# Keep unrelated prompts quiet; matching GitHub requests receive a workflow hint without changing host authorization.
if ($payload -match '(?i)github|issue|pull[ -]?request|review|merge') {
  [Console]::Out.WriteLine('{"hookSpecificOutput":{"hookEventName":"UserPromptSubmit","additionalContext":"Codexy GitHub workflow is installed. Use $git-workflow; GitHub authorization remains with the host, connector, and GitHub."}}')
}
