use serde_json::{Value, json};
use std::{
    fs,
    process::{Command, Output},
};

use super::{
    TestResult,
    fixture::{Fixture, git, success},
    receipt::{receipt_with_identity, receipt_without_watcher, refresh_payload_manifest_digest},
};

#[test]
fn workflow_receipt_checks_accept_complete_and_absent_watcher_classes() -> TestResult {
    let complete = receipt_with_identity(&"a".repeat(40), &"b".repeat(40), 42, 1);
    let output = run_workflow_receipt_checks(&complete, None, None)?;
    assert!(
        output.status.success(),
        "complete receipt was rejected: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let absent = receipt_without_watcher(&"a".repeat(40), &"b".repeat(40));
    let output = run_workflow_receipt_checks(&absent, None, None)?;
    assert!(
        output.status.success(),
        "absent Watcher receipt was rejected with exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let fixture = Fixture::new("no-watcher")?;
    success(fixture.run("no-watcher")?)?;
    Ok(())
}

#[test]
fn workflow_receipt_checks_reject_partial_malformed_and_bad_watcher_inventory() -> TestResult {
    let valid = receipt_with_identity(&"a".repeat(40), &"b".repeat(40), 42, 1);
    let mut partial = valid.clone();
    partial["candidate"]["classes"]["coreWatcherMcp"]["platforms"]
        .as_object_mut()
        .ok_or("watcher platforms")?
        .remove("linux-x86_64")
        .ok_or("linux watcher")?;
    let mut malformed = valid.clone();
    malformed["candidate"]["classes"]["coreWatcherMcp"] = json!("malformed");
    let mut bad_digest = valid;
    bad_digest["candidate"]["classes"]["coreWatcherMcp"]["platforms"]["linux-x86_64"]
        ["sha256"] = json!("not-a-sha256");
    let mut bad_path = receipt_with_identity(&"a".repeat(40), &"b".repeat(40), 42, 1);
    bad_path["candidate"]["classes"]["coreWatcherMcp"]["platforms"]["linux-x86_64"]
        ["path"] = json!("runtime/unexpected-watcher.bin");
    for receipt in [partial, malformed, bad_digest, bad_path] {
        let output = run_workflow_receipt_checks(&receipt, None, None)?;
        assert!(!output.status.success(), "accepted invalid Watcher inventory");
    }
    Ok(())
}

#[test]
fn workflow_receipt_checks_keep_source_tree_and_provenance_guards() -> TestResult {
    let valid = receipt_without_watcher(&"a".repeat(40), &"b".repeat(40));
    let output = run_workflow_receipt_checks(&valid, Some(&"c".repeat(40)), None)?;
    assert!(!output.status.success(), "accepted a mismatched source commit");

    let mut bad_tree = valid.clone();
    bad_tree["candidate"]["source"]["tree"] = json!("not-a-git-tree");
    let output = run_workflow_receipt_checks(&bad_tree, None, None)?;
    assert!(!output.status.success(), "accepted a malformed source tree");

    let mut staged_provenance = valid["provenance"].clone();
    staged_provenance["runId"] = json!(43);
    let output = run_workflow_receipt_checks(&valid, None, Some(staged_provenance))?;
    assert!(!output.status.success(), "accepted mismatched activation provenance");
    Ok(())
}

#[test]
fn absent_watcher_does_not_skip_handoff_or_devtools_digest_validation() -> TestResult {
    let fixture = Fixture::new("no-watcher")?;
    let source = git(&fixture.repo, &["rev-parse", "HEAD"])?;
    let tree = git(&fixture.repo, &["rev-parse", "HEAD^{tree}"])?;
    for field in ["coreHandoff", "devtoolsMcp"] {
        let mut invalid = receipt_without_watcher(&source, &tree);
        let digest = if field == "coreHandoff" {
            &mut invalid["candidate"]["classes"][field]["platforms"]["linux-x86_64"]["sha256"]
        } else {
            invalid["candidate"]["platforms"]["linux-x86_64"]["lsp"]["sha256"] =
                json!("invalid-digest");
            &mut invalid["candidate"]["classes"][field]["platforms"]["linux-x86_64"]["lsp"]["sha256"]
        };
        *digest = json!("invalid-digest");
        refresh_payload_manifest_digest(&mut invalid);
        let root = tempfile::tempdir()?;
        let path = root.path().join("receipt.json");
        fs::write(&path, serde_json::to_vec(&invalid)?)?;
        let output = Command::new(env!("CARGO_BIN_EXE_codexy-activate-runtime"))
            .args(["--repo-root"])
            .arg(&fixture.repo)
            .args(["--bootstrap-version", codexy_runtime::version::runtime_version()])
            .args(["--candidate-receipt"])
            .arg(path)
            .env("CODEXY_TEST_MODE", "1")
            .output()?;
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success() && diagnostics.contains("digest"),
            "accepted invalid {field} digest: {diagnostics}"
        );
    }
    Ok(())
}

fn run_workflow_receipt_checks(
    receipt: &Value,
    source_commit_override: Option<&str>,
    staged_provenance: Option<Value>,
) -> Result<Output, Box<dyn std::error::Error>> {
    // Reuse the workflow's actual shell lines so receipt fixtures cannot drift from activation behavior.
    let root = tempfile::tempdir()?;
    let staging = root.path().join("codexy-runtime-staging");
    fs::create_dir_all(&staging)?;
    fs::write(
        staging.join("runtime-staging-receipt.json"),
        serde_json::to_vec(receipt)?,
    )?;
    let run_attempt = receipt["candidate"]["artifact"]["stagingRunAttempt"]
        .as_u64()
        .ok_or("staging run attempt")?;
    fs::write(
        staging.join("runtime-staging-run.json"),
        serde_json::to_vec(&json!({"run_attempt": run_attempt}))?,
    )?;
    fs::write(
        staging.join("runtime-staging-provenance.json"),
        serde_json::to_vec(&staged_provenance.unwrap_or_else(|| receipt["provenance"].clone()))?,
    )?;
    let workflow = super::super::workflow("runtime-activation.yml")?;
    let steps = workflow["jobs"]["open-activation-pr"]["steps"]
        .as_sequence()
        .ok_or("activation workflow steps")?;
    let step = steps
        .iter()
        .find(|step| step["name"] == "Build local candidate bootstrap and prove authenticated staging identity")
        .ok_or("activation staging step")?;
    let run = step["run"]
        .as_str()
        .ok_or("activation staging step")?;
    let lines = run.lines().collect::<Vec<_>>();
    let start = lines
        .iter()
        .position(|line| line.trim() == r#"test "$(jq -r .schema "$receipt")" = codexy-runtime-candidate-receipt/v1"#)
        .ok_or("receipt validation start")?;
    let end = lines
        .iter()
        .position(|line| line.trim() == r#"cmp "$RUNNER_TEMP/receipt-provenance.json" "$RUNNER_TEMP/staged-provenance.json""#)
        .ok_or("receipt validation end")?;
    let fragment = lines[start..=end].join("\n");
    let script = format!(
        "set -euo pipefail\nstaging_root=\"$RUNNER_TEMP/codexy-runtime-staging\"\nreceipt=\"$staging_root/runtime-staging-receipt.json\"\n{fragment}"
    );
    let source_commit = source_commit_override.unwrap_or_else(|| {
        receipt["candidate"]["source"]["commit"]
            .as_str()
            .unwrap_or_default()
    });
    let staging_run_id = receipt["candidate"]["artifact"]["stagingRunId"]
        .as_u64()
        .ok_or("staging run id")?
        .to_string();
    Ok(Command::new("bash")
        .args(["-c", &script])
        .env("RUNNER_TEMP", root.path())
        .env("SOURCE_COMMIT", source_commit)
        .env("STAGING_RUN_ID", staging_run_id)
        .output()?)
}
