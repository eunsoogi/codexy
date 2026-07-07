use std::process::Command;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
#[test]
fn validator_rejects_fresh_codex_review_request_with_unresolved_actionable_thread() -> TestResult {
    for handoff in [
        "Next action: request fresh @codex review on the current head.\n",
        "No current-head request exists: request @codex review now.\n",
        "No current-head request exists: please request @codex review now.\n",
        "Codex review state: no current-head request exists: request @codex review now.\n",
        "Codex review state: no current-head request exists. Request exactly one fresh Codex review now.\n",
        "No current-head request exists and the next action is to request exactly one fresh Codex review now.\n",
        "No current-head Codex output exists; ready to request Codex review.\n",
        "Next action: post @codex review on the current head.\n",
        "Next action: comment @codex review on the current head.\n",
        "Next action: @codex review.\n",
        "Next action: `@codex review`.\n",
        "Next action is to @codex review now.\n",
        "Review request: @codex review current head.\n",
        "Next action: send @codex review on the current head.\n",
        "Next action: request review from @codex on the current head.\n",
        "Next action: request a review from @codex on the current head.\n",
        "Next action: request @codex to review the current head.\n",
        "No current-head request exists and the next action is to @codex review now.\n",
        "No current-head request exists so request @codex review now.\n",
        "Requested @codex review.\n",
        "@codex review requested.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, unresolved_thread_pr_state())?;
        assert_failure_contains(
            &output,
            "validator should reject fresh review requests while unresolved actionable review threads remain",
            "unresolved review thread blocks fresh Codex review requests",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_single_fresh_codex_review_request_without_unresolved_threads() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review state: no current-head request or output exists. Request exactly one fresh Codex review now.\n",
        clean_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should preserve the exactly-one fresh Codex review path when no review thread blocks it",
    );
    Ok(())
}

#[test]
fn validator_rejects_fresh_codex_review_request_with_existing_current_head_request() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Current-head @codex review request already has eyes. Request exactly one fresh Codex review now.\n",
        current_head_eyes_request_pr_state(),
    )?;
    assert_failure_contains(
        &output,
        "validator should preserve duplicate current-head Codex review request guard",
        "current-head Codex review activity blocks fresh Codex review requests",
    );
    Ok(())
}

#[test]
fn validator_allows_current_head_request_status_without_fresh_request() -> TestResult {
    for handoff in [
        "Current-head @codex review request is pending.\n",
        "Current-head @codex review request has eyes only.\n",
        "Current-head @codex review request: pending.\n",
        "Current-head @codex review request: has eyes only.\n",
        "Current-head Codex review request is pending; waiting for output.\n",
        "Fresh @codex review requested. Waiting for review output.\n",
        "Fresh @codex review requested; waiting for review output.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, current_head_eyes_request_pr_state())?;
        assert_success(
            &output,
            "validator should not treat request-status nouns as fresh review requests",
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_fresh_codex_review_request_without_review_thread_evidence() -> TestResult {
    for (pr_state, needle) in [
        (missing_review_threads_pr_state(), "missing reviewThreads"),
        (
            paginated_review_threads_pr_state(),
            "pagination hasNextPage true",
        ),
    ] {
        let output = validate_handoff_with_pr_state(
            "Request exactly one fresh Codex review now.\n",
            pr_state,
        )?;
        assert_failure_contains(
            &output,
            "validator should require complete reviewThreads evidence before fresh review requests",
            needle,
        );
    }
    Ok(())
}

#[test]
fn validator_allows_negated_fresh_codex_review_request_with_unresolved_thread() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Next action: do not request fresh @codex review yet because review threads remain unresolved.\n",
        unresolved_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should not block handoffs that prohibit fresh Codex review requests",
    );
    Ok(())
}

#[test]
fn validator_allows_no_request_status_with_negated_next_action() -> TestResult {
    for handoff in [
        "Codex review state: no current-head request exists. Next action: do not request fresh @codex review yet because review threads remain unresolved.\n",
        "Codex review state: no current-head Codex review request exists. Next action: do not request fresh @codex review yet because review threads remain unresolved.\n",
        "Codex review state: no @codex review request exists. Next action: do not request fresh @codex review yet because review threads remain unresolved.\n",
        "Codex review state: not ready to request @codex review because review threads remain unresolved.\n",
        "Before requesting @codex review, inspect PR review threads. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        "Codex review comments: none. Next action: keep waiting on review threads; do not request @codex review yet.\n",
        "Next action: do not post @codex review yet because review threads remain unresolved.\n",
        "Next action: must not comment @codex review yet because review threads remain unresolved.\n",
        "Next action: do not send @codex review yet because review threads remain unresolved.\n",
        "Next action: must not send @codex review yet because review threads remain unresolved.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, unresolved_thread_pr_state())?;
        assert_success(
            &output,
            "validator should not treat no-current-head-request status as a fresh review request",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_fresh_codex_review_request_with_accepted_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Accepted no-change rationale documented for thread PRRT_kwDOWaiting. Preventive adjacent review no-change rationale: inspected functions review_thread_resolution::check and tests validator_codex_review_request_threads; invariants hold because unresolved thread handling still blocks exact-comment-only readiness. Request exactly one fresh Codex review now.\n",
        unresolved_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should allow fresh review when each unresolved thread has accepted no-change rationale",
    );
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
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
fn clean_thread_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}

fn unresolved_thread_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [{
                "id": "PRRT_kwDOWaiting",
                "isResolved": false,
                "isOutdated": false,
                "path": "src/validation/review_thread_resolution.rs",
                "comments": {"nodes": [{"author":{"login":"reviewer"},"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r2"}]}
            }]
        }
    }"#
}
fn current_head_eyes_request_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "REVIEW_REQUIRED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content":"EYES","users":{"totalCount":1}}]
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
fn missing_review_threads_pr_state() -> &'static str {
    r#"{"number":174,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"REVIEW_REQUIRED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#
}

fn paginated_review_threads_pr_state() -> &'static str {
    r#"{"number":174,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"REVIEW_REQUIRED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","reviewThreads":{"pageInfo":{"hasNextPage":true},"nodes":[]}}"#
}
