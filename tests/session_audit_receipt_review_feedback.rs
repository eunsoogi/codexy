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
