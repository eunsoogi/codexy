use std::path::Path;

#[test]
fn windows_selected_candidate_proof_preserves_legacy_public_boundary() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
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
            "codexy-mcp-$server.exe",
            "$server entrypoint differs from its runtime",
        ],
    );
}

#[test]
fn windows_candidate_verifier_creates_its_fresh_extraction_directory() {
    let workflow = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
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
    let root = lines.iter().position(|line| *line == "$root = Join-Path $env:RUNNER_TEMP \"selected-candidate\"").expect("candidate extraction root");
    let create = lines.iter().position(|line| *line == "New-Item -ItemType Directory -Force -Path $root | Out-Null").expect("candidate extraction directory");
    let extract = lines.iter().position(|line| *line == "& $windowsTar -xzf $archive -C $root").expect("candidate archive extraction");

    assert!(legacy_exit < root, "legacy-public must not create or extract a Windows candidate");
    assert!(root < create && create < extract, "candidate extraction directory must exist before tar -C");
}
