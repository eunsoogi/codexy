
#[test]
fn windows_selected_candidate_proof_preserves_legacy_public_boundary() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");

    crate::support::assert_structured_literals(
        &workflow,
        "windows-selected-candidate-proof",
        &[
            "verify-windows-selected-candidate:",
            "Verify immutable native Windows candidate bytes",
            "legacy-public baseline intentionally has no selected Windows candidate",
            "candidate-proven",
            "Get-FileHash -Algorithm SHA256 $archive",
            "System32/tar.exe",
            "codexy-mcp-$server-windows-x86_64.exe",
            "codexy-mcp-devtools.exe",
            "$server runtime duplicates the shared dispatcher",
            "legacy selected archive has no native Windows dispatcher; Windows projection is intentionally unavailable",
            "selected Windows dispatcher missing: $dispatcher",
        ],
    );
}

#[test]
fn windows_candidate_verifier_creates_its_fresh_extraction_directory() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");
    let workflow: serde_yaml::Value = serde_yaml::from_str(&workflow).expect("runtime workflow YAML");
    let verifier = workflow["jobs"]["verify-windows-selected-candidate"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify immutable native Windows candidate bytes"))
        .and_then(|step| step["run"].as_str())
        .expect("native Windows verifier step");
    let lines = verifier.lines().map(str::trim).collect::<Vec<_>>();
    let legacy_exit = lines.iter().position(|line| *line == "exit 0").expect("legacy public exit");
    let dispatcher_boundary = lines.iter().position(|line| *line == "$dispatcher = \"plugins/codexy-devtools/mcp/codexy-mcp-devtools.exe\"").expect("dispatcher compatibility boundary");
    let root = lines.iter().position(|line| *line == "$root = Join-Path $env:RUNNER_TEMP \"selected-candidate\"").expect("candidate extraction root");
    let reject_ambient = lines.iter().position(|line| *line == "if (Test-Path -LiteralPath $root) { throw \"candidate extraction root must be fresh: $root\" }").expect("ambient extraction root rejection");
    let create = lines.iter().position(|line| *line == "New-Item -ItemType Directory -Path $root -ErrorAction Stop | Out-Null").expect("candidate extraction directory");
    let extract = lines.iter().position(|line| *line == "& $windowsTar -xzf $archive -C $root").expect("candidate archive extraction");

    assert!(legacy_exit < root, "legacy-public must not create or extract a Windows candidate");
    assert!(dispatcher_boundary < root, "legacy dispatcher-free archives must stop before extraction");
    assert!(root < reject_ambient && reject_ambient < create && create < extract, "candidate extraction root must be fresh before tar -C");
}

#[test]
fn windows_candidate_verifier_uses_powershell_loop_syntax() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");
    let workflow: serde_yaml::Value = serde_yaml::from_str(&workflow).expect("runtime workflow YAML");
    let verifier = workflow["jobs"]["verify-windows-selected-candidate"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify immutable native Windows candidate bytes"))
        .and_then(|step| step["run"].as_str())
        .expect("native Windows verifier step");
    let lines = verifier.lines().map(str::trim).collect::<Vec<_>>();
    let bash_loop = lines.iter().position(|line| *line == "for server in lsp codegraph; do");
    let powershell_loop = lines
        .iter()
        .position(|line| *line == "foreach ($server in @(\"lsp\", \"codegraph\")) {")
        .expect("PowerShell server loop");
    let runtime = lines
        .iter()
        .position(|line| *line == "$runtime = Join-Path $public \"plugins/codexy-devtools/runtime/codexy-mcp-$server-windows-x86_64.exe\"")
        .expect("loop runtime path");

    assert_eq!(bash_loop, None, "native verifier must not use Bash loop syntax");
    assert!(powershell_loop < runtime, "PowerShell loop must contain runtime verification");
}

#[cfg(windows)]
#[test]
fn windows_candidate_verifier_rejects_an_ambient_extraction_root() {
    let workflow = std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows/plugin-runtime-binaries.yml"),
    )
    .expect("read plugin runtime workflow");
    let workflow: serde_yaml::Value = serde_yaml::from_str(&workflow).expect("runtime workflow YAML");
    let verifier = workflow["jobs"]["verify-windows-selected-candidate"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Verify immutable native Windows candidate bytes"))
        .and_then(|step| step["run"].as_str())
        .expect("native Windows verifier step");
    let root_setup = verifier
        .lines()
        .skip_while(|line| line.trim() != "$root = Join-Path $env:RUNNER_TEMP \"selected-candidate\"")
        .take_while(|line| line.trim() != "$windowsTar = Join-Path $env:SystemRoot \"System32/tar.exe\"")
        .collect::<Vec<_>>()
        .join("\n");
    let runner_temp = tempfile::tempdir().expect("runner temp");
    let fresh = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-Command", &root_setup])
        .env("RUNNER_TEMP", runner_temp.path())
        .output()
        .expect("pwsh starts");
    assert!(fresh.status.success(), "fresh extraction root: {}", String::from_utf8_lossy(&fresh.stderr));
    assert!(runner_temp.path().join("selected-candidate").is_dir());

    let ambient = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-Command", &root_setup])
        .env("RUNNER_TEMP", runner_temp.path())
        .output()
        .expect("pwsh starts");
    assert!(!ambient.status.success(), "ambient extraction root must fail closed");
}
