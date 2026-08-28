use std::{fs, process::Command};

use serde_json::Value;

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const PACKAGED_PROOF_PATHS: &[&str] = &[
    "skills/orchestration/references/execution-budget.md",
    "skills/orchestration/SKILL.md",
    "skills/orchestration/templates/session-audit-proof-receipt.json",
];

#[test]
fn complete_template_receipt_requires_every_promised_evidence_field() -> TestResult {
    let receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
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
    let mut receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
    let proof = super::session_audit_receipt::synthetic_fixture(
        "sanitized-installed-content-equivalence.json",
    )?;

    assert_eq!(receipt["installed"]["contentEquivalent"], true);
    assert_eq!(proof["contentEquivalent"], true);
    for list in [
        &receipt["installed"]["changedFiles"],
        &proof["contentProof"]["sourceChangedFiles"],
        &proof["contentProof"]["installedChangedFiles"],
    ] {
        assert_eq!(proof_paths(list)?, PACKAGED_PROOF_PATHS);
    }
    receipt["installed"]["contentProof"] = proof["contentProof"].clone();
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));

    let mut stale = receipt.clone();
    stale["installed"]["contentProof"]["installedChangedFiles"][1]["sha256"] =
        Value::String("d".repeat(64));
    let output = validate(&stale)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("content equivalence"));

    receipt["installed"]["contentProof"]["installedManifestSha256"] =
        Value::String("d".repeat(64));
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("content equivalence"));
    Ok(())
}

#[test]
fn installed_content_proof_rejects_a_one_byte_tamper_through_public_verifier() -> TestResult {
    let receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
    let valid = validate(&receipt)?;
    assert!(valid.status.success(), "stderr:\n{}", stderr(&valid));

    let original = serde_json::to_vec(&receipt)?;
    let mut tampered = original.clone();
    let marker = b"\"installedChangedFiles\":[{\"path\":\"skills/orchestration/references/execution-budget.md\",\"sha256\":\"";
    let start = tampered
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or("installed content proof marker must exist")?
        + marker.len();
    tampered[start] = b'c';
    assert_eq!(tampered.len(), original.len());
    assert_eq!(
        original
            .iter()
            .zip(tampered.iter())
            .filter(|(left, right)| left != right)
            .count(),
        1
    );

    let temp = tempfile::tempdir()?;
    let path = temp.path().join("tampered-receipt.json");
    fs::write(&path, &tampered)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--receipt")
        .arg(path)
        .output()?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("content equivalence"));
    Ok(())
}

#[test]
fn installed_content_proof_keeps_package_fixtures_and_repository_sources_distinct() -> TestResult {
    let repository = codexy_runtime::paths::repository_root();
    let runtime = codexy_runtime::paths::runtime_package_root();
    assert!(repository.join("plugins/codexy/.codex-plugin/plugin.json").is_file());
    assert!(runtime.join("tests/fixtures/session-audit/controlled-receipt.json").is_file());
    assert!(!runtime.join("plugins/codexy/.codex-plugin/plugin.json").exists());
    assert!(!repository.join("tests/fixtures/session-audit/controlled-receipt.json").exists());

    let receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
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
        "skills/orchestration/../template",
    ] {
        let mut receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
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
    let mut receipt = super::session_audit_receipt::synthetic_fixture("controlled-receipt.json")?;
    receipt["audit"]["comparison"]["before"]["window"]["durationSeconds"] = Value::from(300);
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("equal record-count window"));
    Ok(())
}

#[test]
fn template_selects_only_equal_record_window_fields() -> TestResult {
    let template: Value = serde_json::from_str(include_str!(
        "../../../plugins/codexy/skills/orchestration/templates/session-audit-proof-receipt.json"
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

fn proof_paths(value: &Value) -> TestResult<Vec<&str>> {
    value
        .as_array()
        .ok_or("proof list must be an array")?
        .iter()
        .map(|entry| entry["path"].as_str().ok_or_else(|| "path must be a string".into()))
        .collect()
}
