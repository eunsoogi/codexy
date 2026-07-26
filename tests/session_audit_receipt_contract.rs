use std::{fs, process::Command};

use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
    assert_eq!(
        proof["contentProof"]["sourceChangedFiles"][0]["sha256"],
        sha256(root.join(
            "plugins/codexy/skills/token-efficient-orchestration/templates/session-audit-proof-receipt.json"
        ))?
    );
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
    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(&path)
        .output()?;
    let output = if output.status.success() {
        output
    } else {
        Command::new("sha256sum").arg(path).output()?
    };
    if !output.status.success() {
        return Err("a SHA-256 command must calculate sanitized source digests".into());
    }
    String::from_utf8(output.stdout)?
        .split_ascii_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| "SHA-256 output must include a digest".into())
}
