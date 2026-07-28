use crate::support::release_archive as release_archive_support;

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))
            .expect("runtime workflow");
    let inspector = std::fs::read_to_string(root.join("scripts/inspect-release-archive"))
        .expect("archive inspector");
    let trigger_paths = workflow
        .split("permissions:")
        .next()
        .expect("workflow trigger paths");
    assert_eq!(trigger_paths.matches("plugins/codexy/**").count(), 2);
    for path in [
        "scripts/inspect-release-archive",
        "scripts/check-release-archive-content",
        "scripts/check-release-archive-entries",
    ] {
        assert_eq!(
            trigger_paths.matches(path).count(),
            2,
            "workflow must trigger for {path}"
        );
    }
    release_archive_support::assert_runtime_workflow_contract(&workflow, &inspector);
}

#[test]
fn candidate_selected_package_copies_native_windows_entrypoints() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))
            .expect("runtime workflow");
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(&workflow).expect("runtime workflow YAML");
    let assembly = workflow["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps.iter().find(|step| {
                step["name"] == "Assemble state-aware marketplace package without rebuilding"
            })
        })
        .and_then(|step| step["run"].as_str())
        .expect("package assembly step");
    let candidate = assembly
        .split("candidate-proven)")
        .nth(1)
        .and_then(|case| case.split(";;").next())
        .expect("candidate package branch");
    let lines = candidate.lines().map(str::trim).collect::<Vec<_>>();

    for expected in [
        "mkdir -p \"$staged/mcp\" \"$staged/runtime\"",
        "entrypoint=\"mcp/codexy-mcp-${server}.exe\"",
        "test \"$(digest_file \"$candidate/$entrypoint\")\" = \"$digest\"",
        "cp \"$candidate/$entrypoint\" \"$staged/$entrypoint\"",
        "cmp \"$candidate/$entrypoint\" \"$staged/$entrypoint\"",
    ] {
        assert!(
            candidate
                .lines()
                .map(str::trim)
                .any(|line| line == expected),
            "candidate package branch must include {expected}"
        );
    }
    let copied = lines
        .iter()
        .position(|line| *line == "cp \"$candidate/$entrypoint\" \"$staged/$entrypoint\"")
        .expect("candidate entrypoint copy");
    let inspected = lines
        .iter()
        .position(|line| *line == "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz \"$staged\"")
        .expect("candidate archive inspection");

    assert!(
        copied < inspected,
        "archive inspection must observe copied entrypoints"
    );
}
