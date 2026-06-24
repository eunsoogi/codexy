use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_review_response_waiting_on_thread_not_fixed_or_accepted() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not yet fixed or accepted; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it isn't yet fixed or accepted; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting at src/lib.rs remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting: remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. https://github.com/eunsoogi/codexy/pull/174#discussion_r2: remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, mixed_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should allow explicit waiting evidence for a thread not fixed or accepted",
        );
    }
    Ok(())
}
#[test]
fn validator_rejects_waiting_thread_with_only_half_evidence() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not accepted yet; this lane is not complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed yet; this lane is not complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, mixed_review_thread_pr_state())?;
        assert_failure_contains(
            &output,
            "validator should reject waiting evidence that omits not-fixed or not-accepted evidence",
            "PRRT_kwDOWaiting",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_waiting_rationale_referenced_by_github_url() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. https://github.com/eunsoogi/codexy/pull/174#discussion_r2 remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        mixed_review_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should preserve GitHub discussion URLs while segmenting waiting evidence",
    );
    Ok(())
}

#[test]
fn validator_splits_terminal_period_after_waiting_thread_url() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. https://github.com/eunsoogi/codexy/pull/174#discussion_r2. It remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        mixed_review_thread_pr_state(),
    )?;
    assert_failure_contains(
        &output,
        "validator should not let a bare URL sentence reference a later generic waiting claim",
        "PRRT_kwDOWaiting",
    );
    Ok(())
}

#[test]
fn validator_allows_waiting_rationale_with_contraction_negations() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it isn't fixed or accepted yet; this lane is not complete.\n",
        mixed_review_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should recognize contraction negations in waiting claims",
    );
    Ok(())
}

#[test]
fn validator_allows_waiting_handoff_with_contracted_completion_negation() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane isn't complete.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane isn't yet complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, mixed_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should recognize contracted negation before completion claims",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_waiting_handoff_with_contracted_readiness_negation() -> TestResult {
    for handoff in [
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; we aren't ready for handoff.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; we aren't yet ready for handoff.\n",
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; PR is not currently ready for handoff.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, mixed_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should recognize contracted negation before readiness claims",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_verification_completed_waiting_until_merge() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet. Verification completed. This lane is not complete until merge.\n",
        mixed_review_thread_pr_state(),
    )?;
    assert_success(
        &output,
        "validator should treat verification-completed wording as waiting evidence when the lane is explicitly not complete until merge",
    );
    Ok(())
}

#[test]
fn validator_preserves_eyes_only_codex_review_as_waiting() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Fresh @codex review requested for the current head and has eyes only. Waiting for review output; this lane is not blocked and not complete.\n",
        r#"{"number":174,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"REVIEW_REQUIRED"}"#,
    )?;
    assert_success(
        &output,
        "validator should preserve eyes-only Codex review as a waiting state",
    );
    Ok(())
}

#[test]
fn validator_rejects_ready_handoff_synonyms_with_thread_not_fixed_or_accepted() -> TestResult {
    for ready_claim in [
        "PR ready for parent handoff",
        "PR-readiness handoff",
        "Status: PR-ready",
        "Pull request ready",
        "pull-request-ready handoff",
        "Codex review passed",
        "Codex review approved",
        "Codex review completed",
        "Codex review complete",
    ] {
        let output = validate_handoff_with_pr_state(
            &format!(
                "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet. {ready_claim}.\n"
            ),
            mixed_review_thread_pr_state(),
        )?;
        assert_failure_contains(
            &output,
            "validator should reject readiness while a thread is not fixed or accepted",
            "PRRT_kwDOWaiting",
        );
    }
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
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOFixed",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"author":{"login":"reviewer"},"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r1"}]}
                },
                {
                    "id": "PRRT_kwDOWaiting",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"author":{"login":"reviewer"},"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r2"}]}
                }
            ]
        }
    }"#
}
