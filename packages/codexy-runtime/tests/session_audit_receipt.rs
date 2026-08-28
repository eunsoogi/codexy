use std::{fs, path::Path, process::Command};

use serde_json::{Value, json};

use crate::support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn controlled_comparison_fixture_passes_semantic_validation() -> TestResult {
    let receipt = valid_receipt();
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(response["sessionCount"], 2);
    assert_eq!(response["windowKind"], "equal-record-count");
    Ok(())
}

#[test]
fn receipt_accepts_consistent_equal_record_owner_tree() -> TestResult {
    let receipt = valid_receipt();
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    let response: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(response["valid"], true);
    assert_eq!(response["ownerThreadId"], "owner-486");
    assert_eq!(response["windowKind"], "equal-record-count");
    Ok(())
}

#[test]
fn receipt_accepts_consistent_equal_duration_window() -> TestResult {
    let mut receipt = valid_receipt();
    receipt["audit"]["comparison"]["windowPolicy"] = json!({
        "kind": "equal-duration",
        "beforeDurationSeconds": 300,
        "afterDurationSeconds": 300,
        "comparable": true
    });
    receipt["audit"]["comparison"]["before"]["window"]["durationSeconds"] = json!(300);
    receipt["audit"]["comparison"]["after"]["window"]["durationSeconds"] = json!(300);
    let output = validate(&receipt)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn receipt_rejects_owner_window_digest_and_total_mismatches() -> TestResult {
    let cases = [
        (
            "/audit/comparison/ownerBoundary/ownerThreadId",
            json!("different-owner"),
            "owner boundary",
        ),
        (
            "/audit/comparison/after/window/recordsObserved",
            json!(4),
            "record-count window",
        ),
        (
            "/audit/comparison/before/inputSha256",
            json!("not-a-digest"),
            "SHA-256",
        ),
        (
            "/audit/ownerTreeTotals/toolOutputBytes",
            json!(999),
            "owner-tree totals",
        ),
    ];
    for (pointer, replacement, message) in cases {
        let mut receipt = valid_receipt();
        *receipt.pointer_mut(pointer).expect("valid fixture pointer") = replacement;
        let output = validate(&receipt)?;
        assert!(!output.status.success(), "{pointer} unexpectedly passed");
        assert!(
            stderr(&output).contains(message),
            "{pointer} stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn receipt_rejects_duration_mismatch_and_owner_tree_overflow() -> TestResult {
    let mut duration = valid_receipt();
    duration["audit"]["comparison"]["windowPolicy"] = json!({
        "kind": "equal-duration",
        "beforeDurationSeconds": 300,
        "afterDurationSeconds": 301,
        "comparable": true
    });
    let output = validate(&duration)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("equal-duration window"));

    let mut overflow = valid_receipt();
    overflow["audit"]["ownerTreeSessions"].as_array_mut().unwrap()[0]["toolOutputBytes"] =
        json!(u64::MAX);
    let mut second_session = overflow["audit"]["ownerTreeSessions"][0].clone();
    second_session["sessionId"] = json!("session-b");
    overflow["audit"]["ownerTreeSessions"]
        .as_array_mut()
        .unwrap()
        .push(second_session);
    let output = validate(&overflow)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("overflow"));
    Ok(())
}

#[test]
fn receipt_rejects_local_paths_and_unredacted_command_bodies() -> TestResult {
    let mut receipt = valid_receipt();
    receipt["installed"]["cacheRootRelative"] = json!("/Users/private/cache");
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("relative cache root"));

    let mut receipt = valid_receipt();
    receipt["commandReceipts"][0] = json!({
        "command": "tool --secret private",
        "exitCode": 0
    });
    let output = validate(&receipt)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing field `commandId`"));
    assert!(!stderr(&output).contains("tool --secret private"));
    Ok(())
}

#[test]
fn receipt_rejects_unmodeled_private_content_fields() -> TestResult {
    for pointer in [
        "/audit/prompt",
        "/audit/comparison/windowPolicy/toolBody",
    ] {
        let mut receipt = valid_receipt();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        receipt
            .pointer_mut(parent)
            .and_then(Value::as_object_mut)
            .unwrap()
            .insert(key.to_owned(), json!("private-marker"));
        let output = validate(&receipt)?;
        assert!(!output.status.success(), "{pointer} unexpectedly passed");
        let error = stderr(&output);
        assert!(error.contains("unknown field"), "{pointer} stderr:\n{error}");
        assert!(!error.contains("private-marker"));
    }
    Ok(())
}

fn valid_receipt() -> Value {
    synthetic_fixture("controlled-receipt.json")
        .expect("controlled receipt fixture must be valid")
}

pub(super) fn synthetic_fixture(name: &str) -> TestResult<Value> {
    let mut fixture = session_fixture(name)?;
    let temp = tempfile::tempdir()?;
    let manifest = synthetic_digest(temp.path(), "manifest")?;
    let files = ["execution-budget", "orchestration-skill", "session-audit-template"]
        .into_iter()
        .map(|label| synthetic_digest(temp.path(), label))
        .collect::<TestResult<Vec<_>>>()?;
    let proof = fixture
        .pointer_mut(if name == "controlled-receipt.json" {
            "/installed/contentProof"
        } else {
            "/contentProof"
        })
        .ok_or("content proof must exist")?;
    proof["sourceManifestSha256"] = Value::String(manifest.clone());
    proof["installedManifestSha256"] = Value::String(manifest.clone());
    for (index, digest) in files.iter().enumerate() {
        for list in ["sourceChangedFiles", "installedChangedFiles"] {
            proof[list][index]["sha256"] = Value::String(digest.clone());
        }
    }
    if name == "controlled-receipt.json" {
        fixture["installed"]["manifestSha256"] = Value::String(manifest);
        for (index, digest) in files.iter().enumerate() {
            fixture["installed"]["changedFiles"][index]["sha256"] = Value::String(digest.clone());
        }
        for (pointer, label) in [
            ("/audit/inputSha256", "comparison-input"),
            ("/audit/comparison/before/inputSha256", "before-input"),
            ("/audit/ownerTreeSessions/0/inputSha256", "before-input"),
            ("/audit/comparison/after/inputSha256", "after-input"),
            ("/audit/ownerTreeSessions/1/inputSha256", "after-input"),
        ] {
            let digest = synthetic_digest(temp.path(), label)?;
            *fixture.pointer_mut(pointer).ok_or("receipt digest must exist")? = Value::String(digest);
        }
    }
    Ok(fixture)
}

fn synthetic_digest(root: &Path, label: &str) -> TestResult<String> {
    let path = root.join(label);
    fs::write(&path, format!("codexy synthetic {label}\n"))?;
    Ok(support::sha256_file(&path)?)
}

fn session_fixture(name: &str) -> TestResult<Value> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(serde_json::from_slice(&fs::read(
        root.join("tests/fixtures/session-audit").join(name),
    )?)?)
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
