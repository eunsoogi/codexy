use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_negated_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. No accepted no-change rationale documented for thread PRRT_kwDOExample.\n",
        unresolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject negated no-change rationale text\nstdout:\n{}\nstderr:\n{}",
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

#[test]
fn validator_rejects_incomplete_review_thread_evidence() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. PR ready for parent handoff.\n",
        r#"{
            "number": 134,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "reviewThreads": {"nodes": [{}]}
        }"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should fail closed on incomplete review thread evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("incomplete reviewThreads.nodes"),
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

fn unresolved_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {
            "nodes": [
                {
                    "id": "PRRT_kwDOExample",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {
                        "nodes": [
                            {
                                "url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435613705"
                            }
                        ]
                    }
                }
            ]
        }
    }"#
}
