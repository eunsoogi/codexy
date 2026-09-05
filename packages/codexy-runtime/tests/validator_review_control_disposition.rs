use std::{fs, process::Command};

use crate::support::TestResult;
use serde_json::json;

#[test]
fn finding_disposition_producer_rejects_caller_source_and_capture() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let input = temporary.path().join("input.json");
    let output = temporary.path().join("control.json");
    fs::write(
        &input,
        serde_json::to_vec(&json!({
            "control_state": {
                "schema": "codexy.review-control-state.v1",
                "profile": "strict",
                "post_cap_re_review": {"reason": "authenticated_finding_disposition"}
            },
            "authenticated_finding_disposition": {"caller": "forged"},
            "authenticated_finding_disposition_capture": {"caller": "forged"},
            "finding_disposition": {"caller": "forged"}
        }))?,
    )?;
    let result = Command::new(env!("CARGO_BIN_EXE_codexy-review-control"))
        .args(["--produce-review-control", "--input"])
        .arg(&input)
        .args(["--output"])
        .arg(&output)
        .output()?;
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("caller-supplied external finding source")
    );
    assert!(!output.exists());
    Ok(())
}
