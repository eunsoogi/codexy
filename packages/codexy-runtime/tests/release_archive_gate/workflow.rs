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
fn selected_runtime_source_helper_keeps_modern_and_legacy_boundaries() {
    let root = codexy_runtime::paths::repository_root();
    let helper = std::fs::read_to_string(root.join("scripts/download-selected-runtime-package.sh"))
        .expect("selected runtime source helper");
    for required in [
        "if test -f public-release/runtime-release-receipt.json; then",
        "test \"$(jq -r .release.tag \"$receipt\")\" = \"$RELEASE_TAG\"",
        "public release receipt does not match activated staging identity",
        ": >\"$marker_dir/public-release\"",
        "legacy_release=plugins/codexy-devtools/runtime-release.json",
        "test \"$(jq -er .state \"$legacy_release\")\" = legacy-public",
        "test \"$(jq -er .artifact.tag \"$legacy_release\")\" = \"$RELEASE_TAG\"",
        "expected_url=\"https://github.com/$GITHUB_REPOSITORY/releases/download/$RELEASE_TAG/codexy-marketplace-plugin.tar.gz\"",
        "test \"$url\" = \"$expected_url\"",
        "curl --fail --location \"$url\" -o \"$output\"",
        "test \"$(digest_file \"$output\")\" = \"$digest\"",
        ": >\"$marker_dir/legacy-public\"",
        "scripts/download-runtime-staging-artifact staging",
    ] {
        assert!(helper.contains(required), "helper must include {required}");
    }
    assert!(
        !helper.contains("v1.2.2"),
        "helper must remain version-relative"
    );
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

    assert!(candidate.contains("scripts/materialize-runtime-release-archive"));
    assert!(candidate.contains("dist/selected.tar.gz"));
    assert!(candidate.contains("dist/codexy-marketplace-plugin.tar.gz"));
    assert!(candidate.contains("mkdir -p final-inspect"));
    assert!(candidate.contains("tar -xzf"));
    assert!(candidate.contains("final-inspect"));
    assert!(candidate.contains("scripts/inspect-release-archive"));
    assert!(candidate.contains("public-release"));
    let materialized = lines
        .iter()
        .position(|line| {
            line.contains("scripts/materialize-runtime-release-archive")
                && line.contains("dist/selected.tar.gz")
                && line.contains("dist/codexy-marketplace-plugin.tar.gz")
        })
        .expect("candidate materialization");
    let extracted = lines
        .iter()
        .position(|line| {
            line.contains("tar -xzf")
                && line.contains("dist/codexy-marketplace-plugin.tar.gz")
                && line.contains("final-inspect")
        })
        .expect("public projection extraction");
    let inspected = lines
        .iter()
        .position(|line| {
            line.contains("scripts/inspect-release-archive")
                && line.contains("dist/codexy-marketplace-plugin.tar.gz")
                && line.contains("final-inspect/plugins/codexy-devtools")
                && line.contains("public-release")
        })
        .expect("public projection inspection");

    assert!(
        materialized < extracted && extracted < inspected,
        "public projection must be materialized before inspection"
    );
}
