use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_quoted_codex_review_boilerplate_in_readiness_handoff() -> TestResult {
    let output = validate_handoff_with_pr_state(
        r#"Codex review completed. PR is merge-ready. Maintainer requested leave-open for handoff.

Connector evidence:
Reviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`
Comment "@codex review" to request another review.
"#,
        current_head_output_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "validator should not treat quoted Codex connector boilerplate as a fresh review request\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
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
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content":"EYES","users":{"totalCount":1}}]
        }],
        "reviews": [{
            "body":"Review completed.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`\n\nComment \"@codex review\" to request another review.",
            "state": "COMMENTED",
            "author":{"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z",
            "commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"}
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
