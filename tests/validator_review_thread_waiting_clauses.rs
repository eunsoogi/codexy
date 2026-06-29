use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_waiting_thread_after_unrelated_fixed_thread_clause() -> TestResult {
    for separator in [",", ", but", " and"] {
        let output = validate_handoff_with_pr_state(&format!(
            "Review response: fixed PRRT_kwDOFixed{separator} Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should tie fixed claims to the referenced thread across `{separator}` clause boundaries\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_url_waiting_thread_after_and_clause() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed and https://github.com/eunsoogi/codexy/pull/174#discussion_r2 remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
    )?;

    assert!(
        output.status.success(),
        "validator should split URL-referenced waiting threads after `and`\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_waiting_thread_after_grouped_fixed_url_claim() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed https://github.com/eunsoogi/codexy/pull/174#discussion_r1 and https://github.com/eunsoogi/codexy/pull/174#discussion_r2. https://github.com/eunsoogi/codexy/pull/174#discussion_r2 remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject a waiting claim for a URL thread already grouped under a fixed action\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOWaiting"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    r##"{
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
    }"##
}
