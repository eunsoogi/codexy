use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_ignores_codex_review_comment_status_labels() -> TestResult {
    for handoff in [
        "Codex review comment: none.\n",
        "Codex review comments: none.\n",
        "Codex review comment status: no comments.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, current_head_output_pr_state())?;
        assert!(
            output.status.success(),
            "validator should not treat Codex review comment status labels as fresh review requests\nhandoff: {handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_literal_at_codex_comment_request_detection() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Next action: comment @codex review on the current head.\n",
        current_head_output_pr_state(),
    )?;
    assert!(
        !output.status.success(),
        "validator should still reject duplicate literal @codex comment requests\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("current-head Codex review activity blocks fresh Codex review requests"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn current_head_output_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "reviews": [{
            "body":"Review completed.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
            "state": "COMMENTED",
            "author":{"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z",
            "commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"}
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
