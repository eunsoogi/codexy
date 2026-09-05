[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-f]{40}$')]
    [string]$ExpectedSha,
    [Parameter(Mandatory = $true)]
    [ValidateSet("baseline", "current")]
    [string]$Revision,
    [Parameter(Mandatory = $true)]
    [ValidateSet("child", "subagent", "thread")]
    [string]$Concern,
    [Parameter(Mandatory = $true)]
    [ValidateSet("PreToolUse", "PermissionRequest")]
    [string]$EventName,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [string]$RepoRoot
)

$ErrorActionPreference = "Stop"
$OriginalBaselineSha = "9f8c9e207054294b834ef621ae80eb4a8c963793"
$WarmSamples = 5
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = Join-Path $PSScriptRoot ".."
}
$resolvedRepoRoot = Resolve-Path -LiteralPath $RepoRoot
if (-not (Test-Path -LiteralPath $resolvedRepoRoot.Path -PathType Container)) {
    throw "RepoRoot must be a directory"
}
$RepoRoot = $resolvedRepoRoot.Path
$LauncherNames = @{
    child = "codexy-child-thread-creation.cmd"
    subagent = "codexy-subagent-ownership.cmd"
    thread = "codexy-thread-delivery.cmd"
}
$DiagnosticPrefixes = @{
    child = "CODEXY_CHILD_THREAD_CREATION_"
    subagent = "CODEXY_SUBAGENT_OWNERSHIP_"
    thread = "CODEXY_THREAD_DELIVERY_"
}
$PayloadTemplates = @{
    child = '{"hook_event_name":"__EVENT__","tool_name":"mcp__codex_app__create_thread","tool_input":{"model":"gpt-5.6-luna","thinking":"max"}}'
    subagent = '{"hook_event_name":"__EVENT__","tool_name":"multi_agent_v1__spawn_agent","tool_input":{"agent_type":"codexy-cartographer","message":"Map files only."}}'
    thread = '{"hook_event_name":"__EVENT__","tool_name":"mcp__codex_app__send_message_to_thread","tool_input":{"threadId":"parent","model":"gpt-6-astra","thinking":"medium"}}'
}
$Script:Records = [Collections.Generic.List[object]]::new()

function Get-BytesHash([byte[]]$Bytes) {
    return [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($Bytes)).ToLowerInvariant()
}

function Get-Base64([byte[]]$Bytes) {
    return [Convert]::ToBase64String($Bytes)
}

function Invoke-Hook([byte[]]$Payload) {
    $launcher = Join-Path $RepoRoot ("plugins\codexy\hooks\" + $LauncherNames[$Concern])
    if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "missing launcher" }
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = if ($env:ComSpec) { $env:ComSpec } else { Join-Path $env:SystemRoot "System32\cmd.exe" }
    $psi.Arguments = '/d /s /c ""' + $launcher + '" ' + $EventName + '"'
    $psi.WorkingDirectory = $RepoRoot
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment["CODEXY_HOOK_SILENT"] = "1"
    foreach ($name in @("PYTHONHOME", "PYTHONPATH", "PYTHONINSPECT", "PYTHONSTARTUP")) {
        [void]$psi.Environment.Remove($name)
    }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $psi
    $stdout = [IO.MemoryStream]::new()
    $stderr = [IO.MemoryStream]::new()
    $started = $false
    try {
        $watch = [Diagnostics.Stopwatch]::StartNew()
        $started = $process.Start()
        if (-not $started) { throw "launcher process did not start" }
        $stdoutTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
        $stderrTask = $process.StandardError.BaseStream.CopyToAsync($stderr)
        $process.StandardInput.BaseStream.Write($Payload, 0, $Payload.Length)
        $process.StandardInput.Close()
        if (-not $process.WaitForExit(5000)) {
            try { $process.Kill($true) } catch { $process.Kill() }
            throw "launcher exceeded the five-second hook limit"
        }
        $process.WaitForExit()
        $stdoutTask.GetAwaiter().GetResult()
        $stderrTask.GetAwaiter().GetResult()
        $watch.Stop()
        return [pscustomobject]@{
            exit_code = $process.ExitCode
            elapsed_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 3)
            stdout = $stdout.ToArray()
            stderr = $stderr.ToArray()
        }
    } finally {
        if ($started -and -not $process.HasExited) {
            try { $process.Kill($true) } catch { Write-Verbose "process already exited" }
        }
        $stdout.Dispose()
        $stderr.Dispose()
        $process.Dispose()
    }
}

function Assert-Allowed($Result, [string]$Kind) {
    if ($Result.exit_code -ne 0 -or $Result.stdout.Length -ne 0 -or $Result.stderr.Length -ne 0) {
        throw "$Kind allowed path failed: exit=$($Result.exit_code), stdout=$($Result.stdout.Length), stderr=$($Result.stderr.Length)"
    }
}

function Get-SampleRecord([string]$Kind, [int]$Index, $Result, [string]$PayloadHash, [int]$PayloadBytes) {
    return [ordered]@{
        schema = "codexy.hook-startup-measurement.v1"
        record_type = "sample"
        revision = $Revision
        git_sha = $ExpectedSha
        concern = $Concern
        event = $EventName
        sample_kind = $Kind
        sample_index = $Index
        runner_os = $env:RUNNER_OS
        runner_image = "windows-latest"
        payload_bytes = $PayloadBytes
        payload_sha256 = $PayloadHash
        elapsed_ms = $Result.elapsed_ms
        exit_code = $Result.exit_code
        stdout_bytes = $Result.stdout.Length
        stdout_sha256 = Get-BytesHash $Result.stdout
        stdout_b64 = Get-Base64 $Result.stdout
        stderr_bytes = $Result.stderr.Length
        stderr_sha256 = Get-BytesHash $Result.stderr
        stderr_b64 = Get-Base64 $Result.stderr
        allowed = $true
    }
}

function Get-DenyRecord($Result, [string]$PayloadHash, [int]$PayloadBytes) {
    if ($Result.exit_code -ne 0 -or $Result.stderr.Length -ne 0) {
        throw "deny control failed: exit=$($Result.exit_code), stderr=$($Result.stderr.Length)"
    }
    try { $document = ([Text.Encoding]::UTF8.GetString($Result.stdout) | ConvertFrom-Json -Depth 8) }
    catch { throw "deny control returned invalid JSON" }
    $specific = $document.hookSpecificOutput
    if ($null -eq $specific -or $specific.hookEventName -ne $EventName) { throw "deny control returned the wrong event" }
    if ($EventName -eq "PreToolUse") {
        $decision = $specific.permissionDecision
        $reason = [string]$specific.permissionDecisionReason
    } else {
        $decision = $specific.decision.behavior
        $reason = [string]$specific.decision.message
    }
    $prefix = $DiagnosticPrefixes[$Concern] + "ENVELOPE"
    if ($decision -ne "deny" -or -not $reason.StartsWith($prefix)) { throw "deny control returned the wrong native decision" }
    return [ordered]@{
        schema = "codexy.hook-startup-measurement.v1"
        record_type = "control"
        control_type = "deny"
        revision = $Revision
        git_sha = $ExpectedSha
        concern = $Concern
        event = $EventName
        payload_bytes = $PayloadBytes
        payload_sha256 = $PayloadHash
        exit_code = $Result.exit_code
        stdout_bytes = $Result.stdout.Length
        stdout_sha256 = Get-BytesHash $Result.stdout
        stdout_b64 = Get-Base64 $Result.stdout
        stderr_bytes = $Result.stderr.Length
        stderr_sha256 = Get-BytesHash $Result.stderr
        stderr_b64 = Get-Base64 $Result.stderr
        hook_event_name = $specific.hookEventName
        decision = $decision
        reason_prefix = $prefix
        reason_prefix_match = $true
        allowed = $false
    }
}

function Save-Record([string]$Path) {
    $directory = Split-Path -Parent $Path
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    $lines = foreach ($record in $Script:Records) { $record | ConvertTo-Json -Compress -Depth 8 }
    [IO.File]::WriteAllLines($Path, [string[]]$lines, [Text.UTF8Encoding]::new($false))
}

$actualSha = (git -C $RepoRoot rev-parse HEAD).Trim().ToLowerInvariant()
if ($LASTEXITCODE -ne 0 -or $actualSha -ne $ExpectedSha) { throw "script checkout identity mismatch" }
if ((git -C $RepoRoot status --porcelain) -ne "") { throw "script checkout is not clean" }
$payloadText = $PayloadTemplates[$Concern].Replace("__EVENT__", $EventName)
$payload = [Text.Encoding]::UTF8.GetBytes($payloadText)
if ($payload.Length -gt 1024 * 1024) { throw "measurement payload exceeds the hook input limit" }
$payloadHash = Get-BytesHash $payload
[void]$Script:Records.Add([ordered]@{
    schema = "codexy.hook-startup-measurement.v1"
    record_type = "manifest"
    original_baseline_sha = $OriginalBaselineSha
    revision = $Revision
    git_sha = $ExpectedSha
    concern = $Concern
    event = $EventName
    runner_os = $env:RUNNER_OS
    runner_image = "windows-latest"
    runner_name = $env:RUNNER_NAME
    image_os = $env:ImageOS
    image_version = $env:ImageVersion
    processor_architecture = $env:PROCESSOR_ARCHITECTURE
    powershell_version = $PSVersionTable.PSVersion.ToString()
    hook_environment = "CODEXY_HOOK_SILENT=1;Python startup variables removed"
    payload_bytes = $payload.Length
    payload_sha256 = $payloadHash
    payload_b64 = Get-Base64 $payload
    cold_definition = "first hook invocation after checkout on a fresh windows-latest runner"
    warm_definition = "five fresh hook processes after one unrecorded prime"
    warm_samples = $WarmSamples
})

try {
    $cold = Invoke-Hook $payload
    Assert-Allowed $cold "cold"
    [void]$Script:Records.Add((Get-SampleRecord "cold" 0 $cold $payloadHash $payload.Length))
    $prime = Invoke-Hook $payload
    Assert-Allowed $prime "warm prime"
    for ($index = 1; $index -le $WarmSamples; $index++) {
        $warm = Invoke-Hook $payload
        Assert-Allowed $warm "warm $index"
        [void]$Script:Records.Add((Get-SampleRecord "warm" $index $warm $payloadHash $payload.Length))
    }
    $denyPayload = [Text.Encoding]::UTF8.GetBytes('{"hook_event_name":"' + $EventName + '","tool_name":"codexy_measurement_invalid_tool","tool_input":{}}')
    [void]$Script:Records.Add((Get-DenyRecord (Invoke-Hook $denyPayload) (Get-BytesHash $denyPayload) $denyPayload.Length))
} catch {
    [void]$Script:Records.Add([ordered]@{ record_type = "error"; revision = $Revision; git_sha = $ExpectedSha; concern = $Concern; event = $EventName; message = $_.Exception.Message })
    throw
} finally {
    Save-Record $OutputPath
}
