use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_semicolon_linked_waiting_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved; it is not fixed or accepted yet. This lane is not complete.\n",
    )?;
    assert_success(
        &output,
        "validator should keep semicolon-linked unresolved and not-fixed/not-accepted waiting evidence together",
    );
    Ok(())
}

#[test]
fn validator_keeps_semicolon_waiting_evidence_thread_local() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved; thread PRRT_kwDOResolved is not fixed or accepted yet. This lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. https://github.com/eunsoogi/codexy/pull/174#discussion_r2 remains unresolved; https://github.com/eunsoogi/codexy/pull/174#discussion_r3 is not fixed or accepted yet. This lane is not complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff)?;
        assert_failure_contains(
            &output,
            "validator should not let semicolon-delimited evidence from another thread satisfy the waiting thread",
            "PRRT_kwDOWaiting",
        );
    }
    Ok(())
}

#[test]
fn validator_keeps_comma_while_waiting_evidence_thread_local() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOFixed remains unresolved because it is not fixed or accepted, while PRRT_kwDOWaiting remains unresolved because it is not accepted. This lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. PRRT_kwDOResolved remains unresolved; it is not fixed or accepted, while PRRT_kwDOWaiting remains unresolved because it is not accepted. This lane is not complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff)?;
        assert_failure_contains(
            &output,
            "validator should not let waiting evidence from one while clause satisfy another thread",
            "PRRT_kwDOWaiting",
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_semicolon_waiting_rationale_before_later_thread_ref() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved; it is not fixed or accepted, while PRRT_kwDOResolved remains unresolved because it is not fixed or accepted. This lane is not complete.\n",
    )?;
    assert_success(
        &output,
        "validator should preserve carried waiting evidence before a later different thread reference",
    );
    Ok(())
}

#[test]
fn validator_allows_grouped_waiting_rationale_for_each_thread() -> TestResult {
    let output = validate_handoff_with_pr_state_and_state(
        "Review response: fixed PRRT_kwDOFixed. Threads PRRT_kwDOWaiting and PRRT_kwDOOtherWaiting remain unresolved because they are not fixed or accepted yet. This lane is not complete.\n",
        grouped_waiting_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should apply grouped waiting rationale to each referenced unresolved thread",
    );
    Ok(())
}

#[test]
fn validator_allows_currently_not_complete_verification_wait_until_merge() -> TestResult {
    let output = validate_handoff_with_pr_state_and_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet. Verification completed: focused tests passed. This lane is not currently complete until merge.\n",
        codexy_waiting_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should treat currently-not-complete verification wording as waiting evidence",
    );

    let output = validate_handoff_with_pr_state_and_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet. Verification completed: focused tests passed. This lane is complete.\n",
        codexy_waiting_pr_state(),
    )?;
    assert_failure_contains(
        &output,
        "validator should still reject affirmative completion claims with unresolved waiting threads",
        "PRRT_kwDOWaiting",
    );
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state_and_state(handoff, mixed_review_thread_pr_state())
}

fn validate_handoff_with_pr_state_and_state(handoff: &str, pr_state: &str) -> OutputResult {
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

fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &std::process::Output, message: &str, needle: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
                },
                {
                    "id": "PRRT_kwDOResolved",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r3"}]}
                }
            ]
        }
    }"#
}

fn grouped_waiting_pr_state() -> &'static str {
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
                },
                {
                    "id": "PRRT_kwDOOtherWaiting",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r3"}]}
                }
            ]
        }
    }"#
}

fn codexy_waiting_pr_state() -> &'static str {
    r#"{
        "number": 177,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "repository": "eunsoogi/codexy",
        "labels": [],
        "closingIssuesReferences": [{"number": 174, "labels": []}],
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
