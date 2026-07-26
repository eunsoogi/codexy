use std::{fs, process::Command};

use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

#[test]
fn receipt_rejects_observations_that_do_not_match_owner_tree_sessions() -> TestResult {
    for (pointer, replacement) in [
        (
            "/audit/comparison/before/latestCumulativeTokens",
            json!(999),
        ),
        ("/audit/comparison/after/window/turnEvents", json!(999)),
        (
            "/audit/comparison/after/sessionId",
            json!("different-session"),
        ),
        (
            "/audit/comparison/after/inputSha256",
            json!("a".repeat(64)),
        ),
    ] {
        let mut receipt = valid_receipt();
        *receipt.pointer_mut(pointer).ok_or("fixture pointer must exist")? = replacement;
        let output = validate(&receipt)?;
        assert!(!output.status.success(), "{pointer} unexpectedly passed");
        assert!(
            stderr(&output).contains("owner-tree session"),
            "{pointer} stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn receipt_rejects_cross_platform_absolute_cache_paths() -> TestResult {
    for path in ["C:\\Users\\example\\plugins", "C:/example/plugins", "cache\\plugins"] {
        let mut receipt = valid_receipt();
        receipt["installed"]["cacheRootRelative"] = json!(path);
        let output = validate(&receipt)?;
        assert!(!output.status.success(), "unsafe cache path unexpectedly passed");
        assert!(stderr(&output).contains("relative cache root"));
    }
    Ok(())
}

#[test]
fn receipt_rejects_oversized_input_before_decoding() -> TestResult {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("oversized-receipt.json");
    fs::write(&path, vec![b' '; MAX_INPUT_BYTES + 1])?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-session-audit"))
        .arg("--receipt")
        .arg(path)
        .output()?;
    assert!(!output.status.success(), "oversized receipt unexpectedly passed");
    assert!(stderr(&output).contains("receipt input exceeds"));
    Ok(())
}

#[test]
fn receipt_rejects_unbound_owner_session_digests_duplicate_observations_and_drive_paths(
) -> TestResult {
    let mut digest = valid_receipt();
    let mut extra = digest["audit"]["ownerTreeSessions"][1].clone();
    extra["sessionId"] = json!("third-owner-session");
    extra["inputSha256"] = json!("private transcript");
    digest["audit"]["ownerTreeSessions"]
        .as_array_mut()
        .ok_or("owner tree sessions must be an array")?
        .push(extra);
    digest["audit"]["ownerTreeTotals"] = json!({
        "sessionCount": 3,
        "recordsObserved": 6,
        "turnEvents": 6,
        "cumulativeTokens": 500,
        "toolInputBytes": 67,
        "toolOutputBytes": 255,
        "execFamily": {"calls": 6, "inputBytes": 52, "outputBytes": 160},
        "waitFamily": {"calls": 6, "inputBytes": 15, "outputBytes": 95}
    });
    let output = validate(&digest)?;
    assert!(!output.status.success(), "unsafe owner digest unexpectedly passed");
    assert!(stderr(&output).contains("SHA-256"));

    let mut duplicate = valid_receipt();
    duplicate["audit"]["comparison"]["after"] =
        duplicate["audit"]["comparison"]["before"].clone();
    let output = validate(&duplicate)?;
    assert!(!output.status.success(), "duplicate observations unexpectedly passed");
    assert!(stderr(&output).contains("distinct"));

    let mut drive_paths = valid_receipt();
    for pointer in [
        "/installed/changedFiles/0/path",
        "/installed/contentProof/sourceChangedFiles/0/path",
        "/installed/contentProof/installedChangedFiles/0/path",
    ] {
        *drive_paths
            .pointer_mut(pointer)
            .ok_or("changed-file fixture pointer must exist")? = json!("C:/Users/private/file");
    }
    let output = validate(&drive_paths)?;
    assert!(!output.status.success(), "drive-qualified path unexpectedly passed");
    assert!(stderr(&output).contains("safe repository-relative"));
    Ok(())
}

fn valid_receipt() -> Value {
    serde_json::from_str(include_str!("fixtures/session-audit/controlled-receipt.json"))
        .expect("controlled receipt fixture must be valid JSON")
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
