param(
    [Parameter(Mandatory)][ValidateSet("build", "measure")][string]$Mode,
    [Parameter(Mandatory)][string]$BaselineSha,
    [Parameter(Mandatory)][string]$CandidateSha,
    [ValidateSet("baseline", "candidate")][string]$Revision = "baseline",
    [ValidateSet("baseline-first", "candidate-first")][string]$Order = "baseline-first"
)
$ErrorActionPreference = "Stop"
$workspace = $env:GITHUB_WORKSPACE
$controller = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$outputRoot = Join-Path $env:RUNNER_TEMP "governance-measurement"
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
foreach ($sha in @($BaselineSha, $CandidateSha)) {
    if ($sha -cnotmatch '\A[0-9a-f]{40}\z') { throw "exact lowercase source SHA required" }
}
if ($BaselineSha -eq $CandidateSha) { throw "comparison sources must differ" }

function Assert-Source([string]$Root, [string]$Sha) {
    $actual = (& git -C $Root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -ne $Sha) { throw "source identity mismatch" }
    $dirty = @(& git -C $Root status --porcelain --untracked-files=no)
    if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) { throw "source has tracked modifications" }
}
function Get-Source([string]$Name) {
    $root = (Resolve-Path -LiteralPath (Join-Path $workspace ".measurement-$Name")).Path
    $sha = if ($Name -eq "baseline") { $BaselineSha } else { $CandidateSha }
    Assert-Source $root $sha
    return @{ Root = $root; Sha = $sha }
}
function Get-Identity {
    $identity = @(& rustc -vV) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Rust identity unavailable" }
    return $identity
}
function Assert-ExecutablePath([string]$Relative) {
    if ($Relative -cnotmatch '\Apackages/codexy-runtime/target/debug/(?:deps/)?[A-Za-z0-9_.-]+\.exe\z') {
        throw "unexpected artifact executable path"
    }
}
Assert-Source $controller $CandidateSha

if ($Mode -eq "build") {
    $source = Get-Source $Revision
    Set-Location -LiteralPath $source.Root
    $target = Join-Path $source.Root "packages/codexy-runtime/target"
    if (Test-Path -LiteralPath $target) { throw "cold target must be absent" }
    $setup = [Diagnostics.Stopwatch]::StartNew()
    & "$controller/scripts/install-windows-test-prerequisites.ps1"
    & "$controller/scripts/ensure-rust-toolchain.ps1"
    $setup.Stop()
    $identity = Get-Identity
    $env:CARGO_TARGET_DIR = $target
    $build = [Diagnostics.Stopwatch]::StartNew()
    & cargo test --locked --manifest-path packages/codexy-runtime/Cargo.toml --test suite_governance --no-run --message-format=json 1> "$outputRoot/build.jsonl" 2> "$outputRoot/build.log"
    $buildExit = $LASTEXITCODE
    $build.Stop()
    if ($buildExit -ne 0) { Get-Content "$outputRoot/build.log"; throw "build failed" }
    Assert-Source $source.Root $source.Sha
    $artifacts = @(Get-Content "$outputRoot/build.jsonl" | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object {
        $_.reason -eq "compiler-artifact" -and $_.executable
    })
    $files = @()
    $testPath = $null
    foreach ($artifact in $artifacts) {
        $relative = [IO.Path]::GetRelativePath($source.Root, $artifact.executable).Replace('\', '/')
        Assert-ExecutablePath $relative
        $destination = Join-Path "$outputRoot/payload" $relative
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath $artifact.executable -Destination $destination
        $files += @{ path = $relative; sha256 = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash }
        if ($artifact.target.name -eq "suite_governance" -and $artifact.target.kind -contains "test") {
            if ($null -ne $testPath) { throw "duplicate governance executable" }
            $testPath = $relative
        }
    }
    if ($null -eq $testPath) { throw "governance executable missing" }
    @{
        sourceSha = $source.Sha; sourceRoot = $source.Root; revision = $Revision
        sourceTree = (& git rev-parse 'HEAD^{tree}').Trim(); rustc = $identity
        setupSeconds = $setup.Elapsed.TotalSeconds; buildSeconds = $build.Elapsed.TotalSeconds
        cacheCondition = "cold-target-no-actions-cache"; testExecutable = $testPath; files = $files
    } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath "$outputRoot/manifest.json"
    Get-Content -LiteralPath "$outputRoot/manifest.json"
    exit 0
}

& "$controller/scripts/install-windows-test-prerequisites.ps1"
& "$controller/scripts/ensure-rust-toolchain.ps1"
$identity = Get-Identity
$names = if ($Order -eq "baseline-first") { @("baseline", "candidate") } else { @("candidate", "baseline") }
$expectedTests = $null
foreach ($name in $names) {
    $source = Get-Source $name
    $artifactRoot = Join-Path $env:RUNNER_TEMP "governance-$name"
    $manifest = Get-Content -LiteralPath "$artifactRoot/manifest.json" -Raw | ConvertFrom-Json
    if ($manifest.sourceSha -ne $source.Sha -or $manifest.sourceRoot -ne $source.Root -or $manifest.revision -ne $name) {
        throw "artifact source identity mismatch"
    }
    if ($manifest.rustc -ne $identity) { throw "build and measurement toolchains differ" }
    Set-Location -LiteralPath $source.Root
    if ((& git rev-parse 'HEAD^{tree}').Trim() -ne $manifest.sourceTree) { throw "source tree mismatch" }
    $seen = @{}
    foreach ($file in $manifest.files) {
        Assert-ExecutablePath $file.path
        if ($seen.ContainsKey($file.path)) { throw "duplicate artifact path" }
        $seen[$file.path] = $true
        $inputFile = Join-Path "$artifactRoot/payload" $file.path
        if ((Get-FileHash -LiteralPath $inputFile -Algorithm SHA256).Hash -ne $file.sha256) { throw "artifact digest mismatch" }
        $destination = Join-Path $source.Root $file.path
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $destination) | Out-Null
        Copy-Item -LiteralPath $inputFile -Destination $destination
    }
    Assert-ExecutablePath $manifest.testExecutable
    if (-not $seen.ContainsKey($manifest.testExecutable)) { throw "unverified test executable" }
    $executable = Join-Path $source.Root $manifest.testExecutable
    $tests = @(& $executable --list | Where-Object { $_ -match ': test$' } | Sort-Object)
    if ($LASTEXITCODE -ne 0 -or $tests.Count -ne 169) { throw "expected full 169-test workload" }
    if ($null -ne $expectedTests -and @(Compare-Object $expectedTests $tests).Count -ne 0) { throw "test workloads differ" }
    $expectedTests = $tests
    $tests | Set-Content -LiteralPath "$outputRoot/$name-tests.txt"
    $trace = Join-Path $outputRoot "$name-git.jsonl"
    $env:GIT_TRACE2_EVENT = $trace
    $watch = [Diagnostics.Stopwatch]::StartNew()
    & $executable *> "$outputRoot/$name-tests.log"
    $testExit = $LASTEXITCODE
    $watch.Stop()
    Remove-Item Env:GIT_TRACE2_EVENT
    $starts = @(Get-Content -LiteralPath $trace | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object { $_.event -eq "start" })
    $summary = Get-Content -LiteralPath "$outputRoot/$name-tests.log" | Where-Object { $_ -match '^test result:' }
    $result = @{
        sourceSha = $source.Sha; order = $Order; revision = $name; testCount = $tests.Count
        testSeconds = $watch.Elapsed.TotalSeconds; exitCode = $testExit; gitStarts = $starts.Count
        testSummary = $summary; buildSeconds = $manifest.buildSeconds; setupSeconds = $manifest.setupSeconds
        cacheCondition = "prebuilt-executables-fresh-process-and-fixtures"; logicalProcessors = [Environment]::ProcessorCount
        sourceRoot = $source.Root; testExecutable = $manifest.testExecutable; files = $manifest.files; rustc = $identity
    }
    $result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath "$outputRoot/$name-result.json"
    Get-Content -LiteralPath "$outputRoot/$name-result.json"
    Assert-Source $source.Root $source.Sha
    if ($testExit -ne 0 -or $summary -notmatch '169 passed; 0 failed; 0 ignored;') { throw "full governance workload failed" }
}
