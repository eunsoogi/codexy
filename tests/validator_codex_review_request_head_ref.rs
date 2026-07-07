use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_fresh_codex_review_request_without_head_ref_oid() -> TestResult {
    for pr_state in [
        missing_head_ref_oid_pr_state(),
        blank_head_ref_oid_pr_state(),
    ] {
        let output = validate_handoff_with_pr_state(
            "Request exactly one fresh Codex review now.\n",
            pr_state,
        )?;
        assert!(
            !output.status.success(),
            "validator should require headRefOid before fresh review requests\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("headRefOid"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            pr_state_path.to_str().ok_or("pr state path")?,
        ])
        .output()?)
}

fn missing_head_ref_oid_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}

fn blank_head_ref_oid_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid":"   ",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
