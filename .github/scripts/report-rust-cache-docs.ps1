param(
    [string]$Suite,
    [ValidateSet("runner", "prepared", "built")][string]$Phase = "prepared"
)
if ($Suite -ne "library and binaries") { return }
$ErrorActionPreference = "Stop"
$started = [System.Diagnostics.Stopwatch]::StartNew()
$toolchain = Join-Path $HOME $env:CODEXY_WINDOWS_TOOLCHAIN_CACHE_SUBPATH
$target = Join-Path $PSScriptRoot "../../packages/codexy-runtime/target"
$compiler = Join-Path $toolchain "bin/rustc.exe"
if ($Phase -ne "built" -and (Test-Path -LiteralPath $compiler -PathType Leaf)) {
    $compilerHash = (Get-FileHash -LiteralPath $compiler -Algorithm SHA256).Hash
    Write-Output "normal-cache-compiler phase=$Phase sha256=$compilerHash"
}
$roots = [ordered]@{ toolchain = $toolchain }
if ($Phase -ne "runner") {
    $roots.registry = Join-Path $HOME ".cargo/registry"
    $roots.target = $target
}
foreach ($entry in $roots.GetEnumerator()) {
    $exists = Test-Path -LiteralPath $entry.Value -PathType Container
    Write-Output "normal-cache-inventory phase=$Phase root=$($entry.Key) exists=$exists"
    if (-not $exists) { continue }
    $root = (Resolve-Path -LiteralPath $entry.Value).Path
    $groups = @{}
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -File -Force) {
        $relative = [System.IO.Path]::GetRelativePath($root, $file.FullName).Replace('\', '/')
        $parts = $relative.Split('/')
        $category = if ($parts.Count -eq 1) { "root-files" } else { $parts[0] }
        if ($entry.Key -eq "toolchain" -and $relative.StartsWith("lib/rustlib/")) {
            $category = if ($parts.Count -gt 3) { "lib/rustlib/$($parts[2])" } else { "lib/rustlib/manifests" }
        } elseif ($entry.Key -eq "toolchain" -and $parts[0] -eq "share" -and $parts.Count -gt 2) {
            $category = "share/$($parts[1])"
        } elseif ($entry.Key -eq "target" -and $parts.Count -gt 1) {
            $category = if ($parts.Count -gt 2) { "$($parts[0])/$($parts[1])" } else { "$($parts[0])/root-files" }
        }
        if (-not $groups.ContainsKey($category)) { $groups[$category] = @{ Files = 0; Bytes = [long]0 } }
        $groups[$category].Files++
        $groups[$category].Bytes += $file.Length
    }
    foreach ($category in $groups.Keys | Sort-Object) {
        $size = $groups[$category]
        Write-Output "normal-cache-inventory phase=$Phase root=$($entry.Key) category=$category files=$($size.Files) bytes=$($size.Bytes)"
    }
}
if ($Phase -eq "built" -and (Test-Path -LiteralPath "$target/debug/deps")) {
    foreach ($binary in Get-ChildItem -LiteralPath "$target/debug" -File -Filter "*.exe") {
        $copies = @(Get-ChildItem -LiteralPath "$target/debug/deps" -File -Filter "$($binary.BaseName)-*.exe" |
            Where-Object { $_.Length -eq $binary.Length })
        if ($copies.Count -eq 0) { continue }
        $hash = (Get-FileHash -LiteralPath $binary.FullName -Algorithm SHA256).Hash
        foreach ($copy in $copies) {
            if ((Get-FileHash -LiteralPath $copy.FullName -Algorithm SHA256).Hash -ne $hash) { continue }
            Write-Output "normal-cache-duplicate binary=$($binary.Name) deps=$($copy.Name) identical_bytes=$($binary.Length)"
            if ($IsWindows) {
                $previousExitCode = $LASTEXITCODE
                $links = @(& fsutil hardlink list $binary.FullName 2>$null)
                $linked = if ($LASTEXITCODE -eq 0) {
                    @($links | Where-Object { $_.Trim().EndsWith("\debug\deps\$($copy.Name)") }).Count -gt 0
                } else { "unknown" }
                $global:LASTEXITCODE = $previousExitCode
                Write-Output "normal-cache-duplicate binary=$($binary.Name) deps=$($copy.Name) hardlink=$linked"
            }
        }
    }
}
Write-Output "normal-cache-inventory phase=$Phase elapsed_ms=$($started.ElapsedMilliseconds)"
