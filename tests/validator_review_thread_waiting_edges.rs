use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_waiting_evidence_that_contradicts_same_thread_action() -> TestResult {
    for action in [
        "fixed",
        "addressed",
        "implemented",
        "resolved",
        "applied",
        "handled",
        "updated",
        "responded",
        "fixes",
        "resolves",
    ] {
        let handoff = format!(
            "Review response: {action} PRRT_kwDOWaiting. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        );
        let output = validate_handoff_with_pr_state(&handoff)?;

        assert!(
            !output.status.success(),
            "validator should reject waiting evidence when the same thread is also claimed {action}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOWaiting"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_negative_pr_readiness_label_before_waiting_evidence() -> TestResult {
    for label in [
        "PR readiness: not ready",
        "PR ready: no",
        "PR ready: false",
        "PR ready: not requested",
    ] {
        let output = validate_handoff_with_pr_state(&format!(
            "{label}. Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should treat negative PR-readiness label `{label}` as waiting evidence\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, mixed_review_thread_pr_state())?;
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

fn mixed_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOFixed",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r1"}]}
                },
                {
                    "id": "PRRT_kwDOWaiting",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r2"}]}
                }
            ]
        }
    }"#
}
