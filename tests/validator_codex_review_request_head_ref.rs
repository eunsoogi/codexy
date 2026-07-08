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

#[test]
fn validator_rejects_fresh_codex_review_request_without_fresh_pr_activity() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        r#"{
            "number": 174,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "REVIEW_REQUIRED",
            "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
            "headRefCommittedDate": "2026-07-05T10:39:00Z",
            "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_contains(&output, "freshly captured PR comments and reviews");
    Ok(())
}

#[test]
fn validator_rejects_fresh_codex_review_request_without_head_commit_date() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        r#"{
            "number": 174,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "REVIEW_REQUIRED",
            "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments": [],
            "reviews": [],
            "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_contains(&output, "headRefCommittedDate");
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

fn assert_rejected_contains(output: &std::process::Output, expected: &str) {
    assert!(
        !output.status.success(),
        "validator should reject fresh review requests without required PR state\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
