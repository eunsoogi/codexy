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
        (
            r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: ```","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
            "validator should reject Codex completion claims with malformed commit footer",
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

#[test]
fn validator_cli_rejects_generic_ready_claim_without_codex_activity() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready.\n",
        r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject generic readiness claims without Codex review activity",
        "current-head Codex review output",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_dash_label_before_merge_ready_claim() -> TestResult {
    for separator in ["-", "--", "—"] {
        let output = validate_handoff_with_pr_state(
            &format!("No blockers {separator} PR is merge-ready.\n"),
            eyes_only_pr_state(),
        )?;
        assert_rejected_with_stderr(
            &output,
            "validator should treat dash-delimited labels as clause boundaries",
            "eyes-only Codex review request",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_allows_hyphenated_negated_ready_claim() -> TestResult {
    for handoff in [
        "PR is not-merge-ready because Codex review is pending.\n",
        "PR ready: not currently ready for handoff because Codex review is pending.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, eyes_only_pr_state())?;
        assert!(
            output.status.success(),
            "validator should not treat negated readiness as an affirmed readiness claim\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn eyes_only_pr_state() -> &'static str {
    r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}]}"#
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
