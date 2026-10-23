use serde_json::{Value, json};
use sha2::Digest;
use std::fs;

use super::Command;
use crate::support::make_executable;

#[test]
fn publisher_identity_accepts_legacy_and_no_watcher_receipts() {
    // Exercise the shared verifier that both candidate assembly and publication invoke.
    let repo = codexy_runtime::paths::repository_root();
    let root = tempfile::tempdir().expect("publisher fixture root");
    let workflow_path = repo.join(".github/workflows/publish-version-release.yml");
    let workflow_source = fs::read_to_string(&workflow_path).expect("publisher workflow");
    for preserved in [
        "scripts/admit-pypi-environment",
        "git merge-base --is-ancestor \"$ACTIVATION_COMMIT\" origin/main",
        "test \"$GITHUB_REF\" = refs/heads/main",
        "scripts/verify_selected_staging_identity.py",
    ] {
        assert!(
            workflow_source.contains(preserved),
            "lost publication guard: {preserved}"
        );
    }
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(&workflow_source).expect("workflow YAML");
    let step = workflow["jobs"]["publish-release"]["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|step| step["name"] == "Verify selected archive identity")
        .unwrap()["run"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(step.contains("test \"$(git rev-parse HEAD)\" = \"$ACTIVATION_COMMIT\""));
    assert!(step.contains("scripts/verify_selected_staging_identity.py"));
    let mut selected: Value = serde_json::from_slice(
        &fs::read(repo.join(".agents/plugins/runtime-activation.json")).unwrap(),
    )
    .unwrap();
    let plugin = root.path().join("plugins/codexy-devtools");
    fs::create_dir_all(plugin.join(".codex-plugin")).unwrap();
    let manifest = repo.join("plugins/codexy-devtools/.codex-plugin/plugin.json");
    fs::copy(&manifest, plugin.join(".codex-plugin/plugin.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&fs::read(manifest).unwrap()).unwrap();
    let archive = root.path().join("staging/codexy-marketplace-plugin.tar.gz");
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    assert!(
        Command::new("tar")
            .args(["-C"])
            .arg_path(root.path())
            .args(["-czf"])
            .arg_path(&archive)
            .arg("plugins/codexy-devtools")
            .status()
            .unwrap()
            .success()
    );
    selected["artifact"]["sha256"] = json!(format!(
        "{:x}",
        sha2::Sha256::digest(fs::read(&archive).unwrap())
    ));
    let stage = root.path().join("staging");
    fs::write(
        stage.join("runtime-staging-run.json"),
        json!({
            "id": selected["candidate"]["artifact"]["stagingRunId"],
            "run_attempt": selected["candidate"]["artifact"]["stagingRunAttempt"],
        })
        .to_string(),
    )
    .unwrap();
    fs::write(
        stage.join("runtime-staging-provenance.json"),
        serde_json::to_vec(&selected["provenance"]).unwrap(),
    )
    .unwrap();
    let helper = root
        .path()
        .join("scripts/verify_selected_staging_identity.py");
    fs::create_dir_all(helper.parent().unwrap()).unwrap();
    fs::copy(
        repo.join("scripts/verify_selected_staging_identity.py"),
        &helper,
    )
    .unwrap();
    make_executable(&helper).unwrap();
    let source = selected["candidate"]["source"]["commit"]
        .as_str()
        .unwrap()
        .to_owned();
    let run = |selection: &Value, candidate: &Value| {
        fs::create_dir_all(root.path().join(".agents/plugins")).unwrap();
        fs::write(
            root.path().join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec(selection).unwrap(),
        )
        .unwrap();
        fs::write(stage.join("runtime-staging-receipt.json"), serde_json::to_vec(&json!({"candidate": candidate, "artifact": selection["artifact"], "provenance": selection["provenance"]})).unwrap()).unwrap();
        Command::new(&helper)
            .arg_path(stage.join("runtime-staging-receipt.json"))
            .arg_path(stage.join("runtime-staging-run.json"))
            .arg_path(&archive)
            .current_dir(root.path())
            .env("STAGING_SOURCE_COMMIT", &source)
            .env(
                "STAGING_RUN_ID",
                selection["candidate"]["artifact"]["stagingRunId"].to_string(),
            )
            .env("TARGET_VERSION", manifest["version"].as_str().unwrap())
            .output()
            .unwrap()
    };
    // Keep legacy Watcher validation independent of the currently selected class set.
    let mut legacy = selected.clone();
    legacy["candidate"]["classes"]["coreWatcherMcp"] = json!({"platforms": {
        "darwin-arm64": {"sha256": "a".repeat(64)},
        "linux-x86_64": {"sha256": "b".repeat(64)},
        "windows-x86_64": {"sha256": "c".repeat(64)}
    }});
    let watcher = legacy["candidate"].clone();
    assert!(run(&legacy, &watcher).status.success());
    let mut bad_watcher = watcher.clone();
    bad_watcher["classes"]["coreWatcherMcp"]["platforms"]["linux-x86_64"]["sha256"] =
        json!("0".repeat(64));
    assert!(
        !run(&legacy, &bad_watcher).status.success(),
        "bad Watcher digest accepted"
    );
    let mut incomplete = watcher.clone();
    incomplete["classes"]["coreWatcherMcp"]["platforms"]
        .as_object_mut()
        .unwrap()
        .remove("windows-x86_64");
    assert!(
        !run(&legacy, &incomplete).status.success(),
        "bad Watcher platforms accepted"
    );
    let mut no_watcher = selected.clone();
    no_watcher["candidate"]["classes"]
        .as_object_mut()
        .unwrap()
        .remove("coreWatcherMcp");
    let no_watcher_candidate = no_watcher["candidate"].clone();
    assert!(
        run(&no_watcher, &no_watcher_candidate).status.success(),
        "no-Watcher rejected"
    );
    assert!(
        !run(&no_watcher, &watcher).status.success(),
        "receipt-only Watcher class accepted"
    );
    assert!(
        !run(&legacy, &no_watcher_candidate).status.success(),
        "selection-only Watcher class accepted"
    );
    for class in ["coreHandoff", "devtoolsMcp"] {
        let mut missing = no_watcher.clone();
        missing["candidate"]["classes"]
            .as_object_mut()
            .unwrap()
            .remove(class);
        let missing_candidate = missing["candidate"].clone();
        assert!(
            !run(&missing, &missing_candidate).status.success(),
            "missing no-Watcher class accepted: {class}"
        );
    }
    let mut different_tree = no_watcher.clone();
    different_tree["candidate"]["source"]["tree"] = json!("f".repeat(40));
    assert!(
        !run(&different_tree, &no_watcher_candidate).status.success(),
        "mismatched selected source tree accepted"
    );
    let mut different_commit = no_watcher_candidate.clone();
    different_commit["source"]["commit"] = json!("e".repeat(40));
    assert!(
        !run(&no_watcher, &different_commit).status.success(),
        "mismatched source commit accepted"
    );
    let mut different_handoff = no_watcher_candidate.clone();
    different_handoff["classes"]["coreHandoff"]["platforms"]["linux-x86_64"]["sha256"] =
        json!("0".repeat(64));
    assert!(
        !run(&no_watcher, &different_handoff).status.success(),
        "mismatched coreHandoff digest accepted"
    );
    let mut different_devtools = no_watcher_candidate.clone();
    different_devtools["classes"]["devtoolsMcp"]["platforms"]["linux-x86_64"]["lsp"]["sha256"] =
        json!("0".repeat(64));
    assert!(
        !run(&no_watcher, &different_devtools).status.success(),
        "mismatched devtools digest accepted"
    );
    let mut bad_archive_digest = no_watcher.clone();
    bad_archive_digest["artifact"]["sha256"] = json!("0".repeat(64));
    assert!(
        !run(&bad_archive_digest, &no_watcher_candidate)
            .status
            .success(),
        "mismatched archive digest accepted"
    );
    let mut bad_provenance = no_watcher.clone();
    bad_provenance["provenance"]["workflowRunUrl"] = json!("https://example.invalid/run");
    assert!(
        !run(&bad_provenance, &no_watcher_candidate).status.success(),
        "mismatched staged provenance accepted"
    );
}
