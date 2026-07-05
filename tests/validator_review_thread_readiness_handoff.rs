use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_ready_handoff_with_unresolved_accepted_no_change_thread() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOExample. Codex review passed on the current head. PR is merge-ready.\n",
        unresolved_accepted_thread_ready_pr_state(),
    )?;
    assert!(
        !output.status.success(),
        "validator should reject readiness claims while accepted no-change threads remain unresolved\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOExample"),
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

fn unresolved_accepted_thread_ready_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "latestReviews": [{
            "body": "Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
            "author": {"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z"
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id": "PRRT_kwDOExample",
            "isResolved": false,
            "isOutdated": false,
            "path": "plugins/codexy/skills/git-workflow/SKILL.md",
            "comments": {"nodes": [{
                "author": {"login":"reviewer"},
                "url": "https://github.com/eunsoogi/codexy/pull/133#discussion_r1"
            }]}
        }]}
    }"#
}
