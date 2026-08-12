use crate::support::release_archive as release_archive_support;

use serde_yaml::Value;

#[test]
fn archive_gate_workflow_covers_every_packaged_surface_and_native_smoke() {
    let root = codexy_runtime::paths::repository_root();
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))
            .expect("runtime workflow");
    let inspector = std::fs::read_to_string(root.join("scripts/inspect-release-archive"))
        .expect("archive inspector");
    let workflow: Value = serde_yaml::from_str(&workflow).expect("runtime workflow YAML");
    for path in [
        "plugins/codexy-devtools/**",
        "scripts/inspect-release-archive",
        "scripts/check-release-archive-content",
        "scripts/check-release-archive-entries",
    ] {
        assert_trigger_path(&workflow, "pull_request", path);
        assert_trigger_path(&workflow, "push", path);
    }
    let source =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))
            .expect("runtime workflow source");
    release_archive_support::assert_runtime_workflow_contract(&source, &inspector);
}

#[test]
fn archive_gate_workflow_rejects_helper_missing_from_either_event_filter() {
    let missing_push = "on:\n  pull_request:\n    paths: [scripts/**]\n  push:\n    paths: []\n";
    let workflow: Value = serde_yaml::from_str(missing_push).expect("workflow YAML");
    assert_trigger_path_result(
        &workflow,
        "pull_request",
        "scripts/check-release-archive-entries",
    )
    .expect("pull request retains helper trigger");
    assert!(
        assert_trigger_path_result(&workflow, "push", "scripts/check-release-archive-entries")
            .is_err(),
        "missing push helper trigger must fail"
    );

    let missing_pull = "on:\n  pull_request:\n    paths: []\n  push:\n    paths: [scripts/**]\n";
    let workflow: Value = serde_yaml::from_str(missing_pull).expect("workflow YAML");
    assert_trigger_path_result(&workflow, "push", "scripts/check-release-archive-entries")
        .expect("push retains helper trigger");
    assert!(
        assert_trigger_path_result(
            &workflow,
            "pull_request",
            "scripts/check-release-archive-entries"
        )
        .is_err(),
        "missing pull request helper trigger must fail"
    );
}

fn assert_trigger_path(workflow: &Value, event: &str, path: &str) {
    assert_trigger_path_result(workflow, event, path)
        .unwrap_or_else(|error| panic!("workflow must trigger {event} for {path}: {error}"));
}

fn assert_trigger_path_result(workflow: &Value, event: &str, path: &str) -> Result<(), String> {
    let paths = workflow["on"][event]["paths"]
        .as_sequence()
        .ok_or_else(|| format!("{event}.paths"))?;
    let count = paths
        .iter()
        .filter(|entry| {
            entry.as_str()
                == Some(if path.starts_with("scripts/") {
                    "scripts/**"
                } else {
                    path
                })
        })
        .count();
    (count == 1)
        .then_some(())
        .ok_or_else(|| format!("{event} has {count} entries for {path}"))
}

#[test]
fn candidate_selected_package_materializes_and_inspects_the_public_projection() {
    let root = codexy_runtime::paths::repository_root();
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
        "scripts/materialize-runtime-release-archive dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz",
        "mkdir -p final-inspect",
        "tar -xzf dist/codexy-marketplace-plugin.tar.gz -C final-inspect",
        "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz final-inspect/plugins/codexy-devtools public-release",
    ] {
        assert!(
            candidate
                .lines()
                .map(str::trim)
                .any(|line| line == expected),
            "candidate package branch must include {expected}"
        );
    }
    let materialized = lines
        .iter()
        .position(|line| *line == "scripts/materialize-runtime-release-archive dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz")
        .expect("candidate materialization");
    let extracted = lines
        .iter()
        .position(|line| *line == "tar -xzf dist/codexy-marketplace-plugin.tar.gz -C final-inspect")
        .expect("public projection extraction");
    let inspected = lines
        .iter()
        .position(|line| *line == "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz final-inspect/plugins/codexy-devtools public-release")
        .expect("public projection inspection");

    assert!(
        materialized < extracted && extracted < inspected,
        "public projection must be materialized before inspection"
    );
}
