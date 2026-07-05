use std::process::Command;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
#[test]
fn validator_rejects_false_clean_synced_pushed_child_handoff() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        r#"{
            "number": 204,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "headRefOid": "1111111111111111111111111111111111111111",
            "latestReviews": [{
                "body": "Didn't find any major issues.\n\nReviewed commit: `1111111111111111111111111111111111111111`",
                "author": {"login": "chatgpt-codex-connector"},
                "submittedAt": "2026-07-03T00:00:00Z"
            }],
            "worktreeStatus": "M src/validation/instruction_policy.rs\n?? tests/validator_role_instruction_policy.rs",
            "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject false clean child handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("child handoff"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_when_merge_state_is_not_clean() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff.\n",
        pr_state_with(
            r#""mergeStateStatus":"DIRTY","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
        ),
        "mergeStateStatus",
    )
}

#[test]
fn validator_rejects_pr_ready_handoff_with_unresolved_thread() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff.\n",
        pr_state_with(
            r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{"id":"PRRT_kwDOOpen","isResolved":false,"isOutdated":false,"path":"src/validation/mod.rs","comments":{"nodes":[{"url":"https://github.com/eunsoogi/codexy/pull/215#discussion_r1"}]}}]}"#,
        ),
        "unresolved review thread",
    )
}

#[test]
fn validator_rejects_pr_ready_handoff_without_review_threads_evidence() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff. Maintainer override: yes.\n",
        r#"{
            "number": 204,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "headRefOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "worktreeStatus": ""
        }"#
        .to_owned(),
        "reviewThreads",
    )
}

#[test]
fn validator_rejects_pr_ready_handoff_without_review_thread_nodes() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff. Maintainer override: yes.\n",
        r#"{
            "number": 204,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "headRefOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "worktreeStatus": "",
            "reviewThreads": {"pageInfo":{"hasNextPage":false}}
        }"#
        .to_owned(),
        "reviewThreads.nodes",
    )
}

#[test]
fn validator_allows_negative_child_handoff_labels_with_blockers() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean. PR ready: no. Parent can merge: no. Pushed: no. Waiting on current blockers.\n",
        &pr_state_with(
            r#""mergeStateStatus":"DIRTY","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
        ),
    )?;

    assert!(
        output.status.success(),
        "negative readiness labels should not be treated as claims\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
#[test]
fn validator_rejects_ready_child_handoff_with_negative_proof_labels() -> TestResult {
    let state = pr_state_with(
        r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
    );
    for handoff in [
        "Child handoff: ready for parent handoff. Branch clean: no. Synced: not verified. Pushed: no. PR-ready: no. Merge-ready: no.\n",
        "Child handoff: ready for parent handoff. Parent can open PR next: no.\n",
        "Child handoff: ready for parent handoff. Clean: no. Synced: yes. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "Child handoff: ready for parent handoff. Clean: not clean. Synced: yes. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "Child handoff: ready for parent handoff. Clean: pending. Synced: yes. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "PR is merge-ready. Branch clean: dirty. Synced: not synced. Pushed: not pushed at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
    ] {
        assert_rejects_child_handoff(handoff, state.clone(), "negative or non-claim")?;
    }
    Ok(())
}
#[test]
fn validator_rejects_synced_handoff_with_pr_head_mismatch() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. Parent can open PR next: yes.\n",
        pr_state_with(
            r#""mergeStateStatus":"CLEAN","headRefOid":"2222222222222222222222222222222222222222","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
        ),
        "headRefOid",
    )
}

#[test]
fn validator_rejects_pushed_handoff_without_comparable_head() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean. Pushed: yes at 068dbb2.\n",
        pr_state_with(
            r#""mergeStateStatus":"CLEAN","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
        ),
        "headRefOid",
    )
}

#[test]
fn validator_rejects_pushed_handoff_with_abbreviated_head_mismatch() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean. Pushed: yes at 2222222.\n",
        pr_state_with(
            r#""mergeStateStatus":"CLEAN","headRefOid":"1111111111111111111111111111111111111111","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
        ),
        "headRefOid",
    )
}

#[test]
fn validator_rejects_pushed_handoff_when_branch_is_ahead() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        pr_state_with(
            "\"mergeStateStatus\":\"CLEAN\",\"headRefOid\":\"068dbb247b7755035223c91ee39f26830f3c1609\",\"worktreeStatus\":\"## codexy/example...origin/codexy/example [ahead 1]\",\"reviewThreads\":{\"pageInfo\":{\"hasNextPage\":false},\"nodes\":[]}",
        ),
        "ahead",
    )
}

#[test]
fn validator_allows_child_handoff_with_matching_clean_evidence() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        r#"{
            "number": 204,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "headRefOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "latestReviews": [{
                "body": "Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`",
                "author": {"login": "chatgpt-codex-connector"},
                "submittedAt": "2026-07-03T00:00:00Z"
            }],
            "worktreeStatus": "",
            "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;

    assert!(
        output.status.success(),
        "validator should allow clean child handoff evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_rejects_child_handoff(handoff: &str, pr_state: String, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, &pr_state)?;
    assert!(
        !output.status.success(),
        "validator should reject false child handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(needle), "unexpected stderr: {stderr}");
    Ok(())
}

fn pr_state_with(fields: &str) -> String {
    format!(
        r#"{{
            "number":204,
            "state":"OPEN",
            "isDraft":false,
            "reviewDecision":"APPROVED",
            "latestReviews":[{{
                "body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`",
                "author":{{"login":"chatgpt-codex-connector"}},
                "submittedAt":"2026-07-03T00:00:00Z"
            }}],
            {fields}
        }}"#
    )
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
