use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_codex_completion_claim_without_current_head_output() -> TestResult {
    for (pr_state, message) in [
        (
            r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
            "validator should reject Codex completion claims without connector output",
        ),
        (
            r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaa`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
            "validator should reject Codex completion claims with only stale connector output",
        ),
    ] {
        let output = validate_handoff_with_pr_state(
            "Codex review approved on the current head. PR is merge-ready.\n",
            pr_state,
        )?;
        assert_rejected_with_stderr(&output, message, "current-head Codex review output");
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_generic_ready_claim_with_only_stale_codex_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready.\n",
        r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaa`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject generic readiness claims with only stale Codex output",
        "current-head Codex review output",
    );
    Ok(())
}

fn assert_rejected_with_stderr(output: &std::process::Output, message: &str, expected: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
