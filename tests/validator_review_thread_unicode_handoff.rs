use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_handles_non_ascii_prefix_before_review_action() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "검토 결과입니다. Review response: addressed current head. PR ready for parent handoff.\n",
        unresolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject unresolved feedback without panicking on non-ASCII prefix\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unresolved review thread"),
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
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

fn unresolved_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOS6i-_86Kho2O",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435435879"}]}
                }
            ]
        }
    }"#
}
