use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_fresh_request_with_unresolved_outdated_thread() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Fixed the outdated Codex thread. Request exactly one fresh Codex review now.\n",
        pr_state_with_unresolved_outdated_thread(),
    )?;
    assert_failure_contains(
        &output,
        "unresolved review thread blocks fresh Codex review requests",
    );
    Ok(())
}

#[test]
fn validator_allows_fresh_request_after_headless_request_has_stale_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        pr_state_with_headless_request_and_later_stale_output(),
    )?;
    assert_success(
        &output,
        "validator should allow fresh review after an old headless request was cleared by stale Codex output",
    );
    Ok(())
}

#[test]
fn validator_rejects_fresh_request_when_current_head_request_has_only_stale_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        pr_state_with_current_head_request_and_later_stale_output(),
    )?;
    assert_failure_contains(
        &output,
        "current-head Codex review activity blocks fresh Codex review requests",
    );
    Ok(())
}

#[test]
fn validator_allows_fresh_request_after_stale_acknowledged_request_without_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        pr_state_with_stale_acknowledged_request_without_output(),
    )?;
    assert_success(
        &output,
        "validator should allow fresh review when the only acknowledged request belongs to an old head",
    );
    Ok(())
}

#[test]
fn validator_treats_requesting_wording_as_fresh_codex_review_request() -> TestResult {
    for handoff in [
        "Requesting fresh Codex review now.\n",
        "I'm requesting @codex review on the current head.\n",
    ] {
        let output =
            validate_handoff_with_pr_state(handoff, pr_state_with_unresolved_outdated_thread())?;
        assert_failure_contains(
            &output,
            "unresolved review thread blocks fresh Codex review requests",
        );
    }
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: String) -> OutputResult {
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

fn assert_failure_contains(output: &std::process::Output, needle: &str) {
    assert!(
        !output.status.success(),
        "validator should reject this fresh review request\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn pr_state_with_unresolved_outdated_thread() -> String {
    serde_json::json!({
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": [{
            "id": "PRRT_kwDOOutdated",
            "isResolved": false,
            "isOutdated": true,
            "path": "src/validation/review_thread_resolution.rs",
            "comments": {"nodes": [{
                "author": {"login": "chatgpt-codex-connector"},
                "url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r3"
            }]}
        }]}
    })
    .to_string()
}

fn pr_state_with_headless_request_and_later_stale_output() -> String {
    serde_json::json!({
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content": "EYES", "users": {"totalCount": 1}}]
        }],
        "latestReviews": [{
            "body": "Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaa`",
            "author": {"login": "chatgpt-codex-connector"},
            "submittedAt": "2026-06-22T12:50:03Z"
        }],
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
    })
    .to_string()
}

fn pr_state_with_current_head_request_and_later_stale_output() -> String {
    let mut pr_state = serde_json::from_str::<serde_json::Value>(
        &pr_state_with_headless_request_and_later_stale_output(),
    )
    .expect("test fixture parses");
    pr_state["comments"][0]["commit"] =
        serde_json::json!({"oid": "32b03a210b3defb2d29dd352283ea2488e60d893"});
    pr_state.to_string()
}

fn pr_state_with_stale_acknowledged_request_without_output() -> String {
    serde_json::json!({
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "commit": {"oid": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
            "reactionGroups": [{"content": "EYES", "users": {"totalCount": 1}}]
        }],
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
    })
    .to_string()
}
