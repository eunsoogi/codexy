use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_generic_readiness_with_codex_output_without_head_ref_oid() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "latestReviews":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(&output, "headRefOid");
    Ok(())
}

#[test]
fn validator_cli_rejects_generic_readiness_with_codex_output_and_blank_head_ref_oid() -> TestResult
{
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"",
            "latestReviews":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(&output, "headRefOid");
    Ok(())
}

fn assert_rejected_with_stderr(output: &std::process::Output, expected: &str) {
    assert!(
        !output.status.success(),
        "validator should fail\nstdout:\n{}\nstderr:\n{}",
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
