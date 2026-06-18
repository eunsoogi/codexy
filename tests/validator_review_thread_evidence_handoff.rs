use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
const NORMAL_OPEN_PR_STATE: &str = r#"{"number":134,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#;
#[test]
fn validator_rejects_negated_no_change_rationale() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. No accepted no-change rationale documented for thread PRRT_kwDOExample.\n",
        unresolved_review_thread_pr_state(),
        "PRRT_kwDOExample",
        "validator should reject negated no-change rationale text",
    )
}
#[test]
fn validator_rejects_post_label_negated_no_change_rationale() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. Accepted no-change rationale was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        unresolved_review_thread_with_id_pr_state("PRRT_kwDOS6i-_86KixXq"),
        "PRRT_kwDOS6i-_86KixXq",
        "validator should reject post-label negated no-change rationale text",
    )
}
#[test]
fn validator_rejects_punctuated_post_label_negated_no_change_rationale() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. Accepted no-change rationale: was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        unresolved_review_thread_with_id_pr_state("PRRT_kwDOS6i-_86KixXq"),
        "PRRT_kwDOS6i-_86KixXq",
        "validator should reject punctuated post-label negated rationale text",
    )
}
#[test]
fn validator_treats_review_comments_as_review_response() -> TestResult {
    assert_handoff_fails(
        "Addressed Codex review comments on the current head.\n",
        NORMAL_OPEN_PR_STATE,
        "reviewThreads.nodes",
        "validator should require reviewThreads.nodes for addressed review comments",
    )
}
#[test]
fn validator_treats_resolved_review_comments_as_review_response() -> TestResult {
    assert_handoff_fails(
        "Resolved Codex review comments on the current head.\n",
        NORMAL_OPEN_PR_STATE,
        "reviewThreads.nodes",
        "validator should require reviewThreads.nodes for resolved review comments",
    )
}
#[test]
fn validator_preserves_review_feedback_context_across_section_breaks() -> TestResult {
    assert_handoff_fails(
        "Review feedback:\n- Addressed all requested changes on the current head.\n",
        NORMAL_OPEN_PR_STATE,
        "reviewThreads.nodes",
        "validator should preserve review-feedback context across section breaks",
    )
}
#[test]
fn validator_preserves_review_feedback_context_across_all_bullets() -> TestResult {
    assert_handoff_fails(
        "Review feedback:\n- Verification rerun.\n- Fixed the Codex comment.\n",
        NORMAL_OPEN_PR_STATE,
        "reviewThreads.nodes",
        "validator should preserve review-feedback context across all bullets",
    )
}
#[test]
fn validator_rejects_incomplete_review_thread_evidence() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. PR ready for parent handoff.\n",
        r#"{
            "number": 134,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "reviewThreads": {"nodes": [{}]}
        }"#,
        "incomplete reviewThreads.nodes",
        "validator should fail closed on incomplete review thread evidence",
    )
}
#[test]
fn validator_allows_clean_codex_review_with_unrelated_fix_without_threads() -> TestResult {
    assert_handoff_succeeds(
        "Codex review passed. Fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
        "validator should not treat unrelated fixes as review feedback responses",
    )
}
#[test]
fn validator_allows_no_review_feedback_with_unrelated_fix_without_threads() -> TestResult {
    assert_handoff_succeeds(
        "Review feedback: none from Codex. Fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
        "validator should not treat unrelated fixes after no-review-feedback text as review feedback responses",
    )
}
#[test]
fn validator_allows_comma_separated_no_review_feedback_with_unrelated_fix() -> TestResult {
    assert_handoff_succeeds(
        "Review feedback: none from Codex, fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
        "validator should not treat comma-separated no-review-feedback text as a response",
    )
}
#[test]
fn validator_limits_negation_to_matched_review_action() -> TestResult {
    assert_handoff_fails(
        "Review feedback: did not change the API, fixed the requested test.\n",
        NORMAL_OPEN_PR_STATE,
        "reviewThreads.nodes",
        "validator should not let unrelated negation suppress the matched action",
    )
}
#[test]
fn validator_rejects_unresolved_outdated_review_thread_after_response() -> TestResult {
    assert_handoff_fails(
        "Review response: fixed the Codex review feedback on the current head.\n",
        outdated_unresolved_review_thread_pr_state(),
        "PRRT_kwDOOutdated",
        "validator should require resolution for addressed outdated review threads",
    )
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: impl AsRef<str>) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state.as_ref())?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}
fn assert_handoff_fails(
    handoff: &str,
    pr_state: impl AsRef<str>,
    needle: &str,
    message: &str,
) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, pr_state)?;
    assert_failure_contains(&output, needle, message);
    Ok(())
}
fn assert_handoff_succeeds(handoff: &str, pr_state: impl AsRef<str>, message: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, pr_state)?;
    assert_success(&output, message);
    Ok(())
}
fn assert_failure_contains(output: &std::process::Output, needle: &str, message: &str) {
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
fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn outdated_unresolved_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {
            "nodes": [
                {
                    "id": "PRRT_kwDOOutdated",
                    "isResolved": false,
                    "isOutdated": true,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {
                        "nodes": [
                            {
                                "url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435715837"
                            }
                        ]
                    }
                }
            ]
        }
    }"#
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
fn unresolved_review_thread_pr_state() -> String {
    unresolved_review_thread_with_id_pr_state("PRRT_kwDOExample")
}
fn unresolved_review_thread_with_id_pr_state(id: &str) -> String {
    format!(
        r#"{{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {{
            "nodes": [
                {{
                    "id": "{id}",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {{
                        "nodes": [
                            {{
                                "url": "https://github.com/eunsoogi/codexy/pull/134#discussion_r3435613705"
                            }}
                        ]
                    }}
                }}
            ]
        }}
    }}"#
    )
}
