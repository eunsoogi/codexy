use std::{path::Path, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
#[test]
fn validator_ignores_copied_footer_comments_before_fresh_codex_review() -> TestResult {
    for comment in [
        r#"Comment "@codex review" to request another review."#,
        r#"- Comment "@codex review" to request another review."#,
        r#"- [x] Comment "@codex review" to request another review."#,
        r#"@codex review request: none yet."#,
    ] {
        let output = validate_handoff_with_pr_state(
            "Request exactly one fresh Codex review now.\n",
            clean_pr_state_with_comment(comment),
        )?;
        assert_success(
            &output,
            "validator should ignore copied connector footer/status comments\ncomment: {comment}\nstdout:\n{}\nstderr:\n{}",
        );
    }
    Ok(())
}
#[test]
fn validator_ignores_negated_pr_comments_before_fresh_codex_review() -> TestResult {
    for comment in [
        "Next action: do not request fresh @codex review yet.",
        "Next action: don't request @codex review until review threads are resolved.",
        "Next action: must not comment @codex review yet because review threads remain unresolved.",
    ] {
        let output = validate_handoff_with_pr_state(
            "Request exactly one fresh Codex review now.\n",
            clean_pr_state_with_comment(comment),
        )?;
        assert_success(
            &output,
            "validator should ignore negated captured Codex request comments\ncomment: {comment}\nstdout:\n{}\nstderr:\n{}",
        );
    }
    Ok(())
}
#[test]
fn validator_preserves_actual_codex_review_comment_duplicate_guard() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        clean_pr_state_with_comment("@codex review"),
    )?;
    assert_failure_contains(
        &output,
        "validator should still reject a duplicate fresh request after a real @codex review comment\nstdout:\n{}\nstderr:\n{}",
        "current-head Codex review activity blocks fresh Codex review requests",
    );
    Ok(())
}
#[test]
fn validator_preserves_acknowledged_split_comment_duplicate_guard() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        clean_pr_state_with_comment(
            "No current-head request exists and the next action is to @codex review now.",
        ),
    )?;
    assert_failure_contains(
        &output,
        "validator should reject a duplicate request after an acknowledged split-action comment\nstdout:\n{}\nstderr:\n{}",
        "current-head Codex review activity blocks fresh Codex review requests",
    );
    Ok(())
}
#[test]
fn validator_clears_no_head_request_after_later_stale_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        clean_pr_state_with_later_stale_codex_output(),
    )?;
    assert_success(
        &output,
        "validator should allow fresh review requests when later stale Codex output clears a no-head issue-comment request\nstdout:\n{}\nstderr:\n{}",
    );
    Ok(())
}

#[test]
fn validator_preserves_current_head_request_after_later_stale_output() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        clean_pr_state_with_current_head_request_and_later_stale_output(),
    )?;
    assert_failure_contains(
        &output,
        "validator should reject duplicate fresh review requests when only stale Codex output follows a current-head request\nstdout:\n{}\nstderr:\n{}",
        "current-head Codex review activity blocks fresh Codex review requests",
    );
    Ok(())
}
#[test]
fn validator_preserves_rest_captured_eyes_request() -> TestResult {
    for reactions in [
        serde_json::json!([{"content": "eyes"}]),
        serde_json::json!({"eyes": 1}),
    ] {
        let output = validate_handoff_with_pr_state(
            "Request exactly one fresh Codex review now.\n",
            clean_pr_state_with_rest_eyes_reaction(reactions),
        )?;
        assert_failure_contains(
            &output,
            "validator should reject duplicate fresh review requests after REST-captured EYES reactions\nstdout:\n{}\nstderr:\n{}",
            "current-head Codex review activity blocks fresh Codex review requests",
        );
    }
    Ok(())
}
#[test]
fn validator_ignores_unacknowledged_codex_review_comment_before_retry() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        clean_pr_state_with_unacknowledged_comment("@codex review"),
    )?;
    assert_success(
        &output,
        "validator should allow retry when prior @codex review comment lacks EYES acknowledgement\nstdout:\n{}\nstderr:\n{}",
    );
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
fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn assert_failure_contains(output: &std::process::Output, message: &str, needle: &str) {
    assert!(!output.status.success(), "{message}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn clean_pr_state_with_comment(comment: &str) -> String {
    serde_json::json!({
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": comment,
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content": "EYES", "users": {"totalCount": 1}}]
        }],
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
    })
    .to_string()
}
fn clean_pr_state_with_later_stale_codex_output() -> String {
    serde_json::json!({
        "number": 174, "state": "OPEN", "isDraft": false,
        "mergeStateStatus": "CLEAN", "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review", "author": {"login": "eunsoogi"},
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

fn clean_pr_state_with_current_head_request_and_later_stale_output() -> String {
    serde_json::json!({
        "number": 174, "state": "OPEN", "isDraft": false,
        "mergeStateStatus": "CLEAN", "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review", "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content": "EYES", "users": {"totalCount": 1}}],
            "commit": {"oid": "32b03a210b3defb2d29dd352283ea2488e60d893"}
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
fn clean_pr_state_with_rest_eyes_reaction(reactions: serde_json::Value) -> String {
    serde_json::json!({
        "number": 174, "state": "OPEN", "isDraft": false,
        "mergeStateStatus": "CLEAN", "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review", "user": {"login": "eunsoogi"},
            "created_at": "2026-06-22T12:45:06Z", "reactions": reactions
        }],
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
    })
    .to_string()
}
fn clean_pr_state_with_unacknowledged_comment(comment: &str) -> String {
    serde_json::json!({
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": comment,
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": []
        }],
        "reviewThreads": {"pageInfo": {"hasNextPage": false}, "nodes": []}
    })
    .to_string()
}
