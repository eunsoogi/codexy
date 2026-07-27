use std::{fs, process::Command};

use serde_json::Value;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PACKAGED_PROOF_PATHS: &[&str] = &[
    "skills/codex-orchestration/references/execution-budget.md",
    "skills/token-efficient-orchestration/SKILL.md",
    "skills/token-efficient-orchestration/templates/session-audit-proof-receipt.json",
];

#[test]
fn complete_template_receipt_requires_every_promised_evidence_field() -> TestResult {
    let receipt: Value = serde_json::from_str(include_str!(
        "fixtures/session-audit/controlled-receipt.json"
    ))?;
    assert!(validate(&receipt)?.status.success());

    for pointer in [
        "/lane/issue",
        "/installed/manifestSha256",
        "/installed/contentEquivalent",
        "/installed/contentProof",
        "/audit/duplicateEventsSkipped",
        "/metrics",
        "/goalPlanReceipts",
        "/commandReceipts/0/exitCode",
    ] {
        let mut incomplete = receipt.clone();
        remove(&mut incomplete, pointer)?;
        let output = validate(&incomplete)?;
        assert!(!output.status.success(), "{pointer} unexpectedly passed");
    }
    Ok(())
}

#[test]
fn sanitized_installed_content_proof_binds_the_receipt() -> TestResult {
    let mut receipt: Value = serde_json::from_str(include_str!(
        "fixtures/session-audit/controlled-receipt.json"
    ))?;
    let proof: Value = serde_json::from_str(include_str!(
        "fixtures/session-audit/sanitized-installed-content-equivalence.json"
    ))?;

    assert_eq!(receipt["installed"]["contentEquivalent"], true);
    assert_eq!(proof["contentEquivalent"], true);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        proof["contentProof"]["sourceManifestSha256"],
        sha256(root.join("plugins/codexy/.codex-plugin/plugin.json"))?
    );
    for list in [
        &receipt["installed"]["changedFiles"],
        &proof["contentProof"]["sourceChangedFiles"],
        &proof["contentProof"]["installedChangedFiles"],
    ] {
        assert_eq!(proof_paths(list)?, PACKAGED_PROOF_PATHS);
    }
    for path in PACKAGED_PROOF_PATHS {
        let digest = sha256(root.join("plugins/codexy").join(path))?;
        assert_eq!(proof_digest(&proof["contentProof"]["sourceChangedFiles"], path)?, digest);
        assert_eq!(proof_digest(&proof["contentProof"]["installedChangedFiles"], path)?, digest);
    }
    receipt["installed"]["contentProof"] = proof["contentProof"].clone();
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));

    receipt["installed"]["contentProof"]["installedManifestSha256"] =
        Value::String("d".repeat(64));
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("content equivalence"));
    Ok(())
}

#[test]
fn shared_sha256_digest_matches_the_known_plugin_manifest_digest() -> TestResult {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/.codex-plugin/plugin.json");
    assert_eq!(support::sha256_file(&path)?, sha256(path)?);
    Ok(())
}

#[test]
fn shared_sha256_digest_reports_missing_input_without_a_host_tool() {
    let missing = tempfile::tempdir().unwrap().path().join("missing-input");
    let error = support::sha256_file(&missing).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn installed_content_proof_rejects_unsafe_path_identities() -> TestResult {
    for path in [
        "/private/template",
        "../template",
        "skills/token-efficient-orchestration/../template",
    ] {
        let mut receipt: Value = serde_json::from_str(include_str!(
            "fixtures/session-audit/controlled-receipt.json"
        ))?;
        for pointer in [
            "/installed/changedFiles",
            "/installed/contentProof/sourceChangedFiles",
            "/installed/contentProof/installedChangedFiles",
        ] {
            receipt
                .pointer_mut(pointer)
                .and_then(Value::as_array_mut)
                .ok_or("proof list must be an array")?[0]["path"] = Value::String(path.to_owned());
        }
        let output = validate(&receipt)?;
        assert!(!output.status.success(), "accepted unsafe path {path:?}");
        assert!(stderr(&output).contains("safe repository-relative"));
    }
    Ok(())
}

#[test]
fn equal_record_receipt_rejects_duration_only_window_keys() -> TestResult {
    let mut receipt: Value = serde_json::from_str(include_str!(
        "fixtures/session-audit/controlled-receipt.json"
    ))?;
    receipt["audit"]["comparison"]["before"]["window"]["durationSeconds"] = Value::from(300);
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("equal record-count window"));
    Ok(())
}

#[test]
fn template_selects_only_equal_record_window_fields() -> TestResult {
    let template: Value = serde_json::from_str(include_str!(
        "../plugins/codexy/skills/token-efficient-orchestration/templates/session-audit-proof-receipt.json"
    ))?;
    let policy = &template["audit"]["comparison"]["windowPolicy"];
    assert_eq!(policy["kind"], "equal-record-count");
    assert!(policy.get("beforeDurationSeconds").is_none());
    assert!(policy.get("afterDurationSeconds").is_none());
    Ok(())
}

fn remove(value: &mut Value, pointer: &str) -> TestResult {
    let (parent, key) = pointer.rsplit_once('/').ok_or("pointer must have a parent")?;
    let parent = value.pointer_mut(parent).ok_or("parent must exist")?;
    if let Some(object) = parent.as_object_mut() {
        object.remove(key);
    } else if let Some(array) = parent.as_array_mut() {
        array.remove(key.parse()?);
    }
    Ok(())
}

fn validate(receipt: &Value) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("receipt.json");
    fs::write(&path, serde_json::to_vec(receipt)?)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--receipt")
        .arg(path)
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn sha256(path: std::path::PathBuf) -> TestResult<String> {
    Ok(support::sha256_file(&path)?)
}

fn proof_paths(value: &Value) -> TestResult<Vec<&str>> {
    value
        .as_array()
        .ok_or("proof list must be an array")?
        .iter()
        .map(|entry| entry["path"].as_str().ok_or_else(|| "path must be a string".into()))
        .collect()
}

fn proof_digest<'a>(value: &'a Value, path: &str) -> TestResult<&'a str> {
    value
        .as_array()
        .ok_or("proof list must be an array")?
        .iter()
        .find(|entry| entry["path"] == path)
        .and_then(|entry| entry["sha256"].as_str())
        .ok_or_else(|| "proof path must include a digest".into())
}
