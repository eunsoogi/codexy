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
    )
}
#[test]
fn validator_rejects_post_label_negated_no_change_rationale() -> TestResult {
    assert_rejects_thread_rationale(
        "Review response: addressed current head. Accepted no-change rationale was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        "PRRT_kwDOS6i-_86KixXq",
    )
}
#[test]
fn validator_rejects_punctuated_post_label_negated_no_change_rationale() -> TestResult {
    assert_rejects_thread_rationale(
        "Review response: addressed current head. Accepted no-change rationale: was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        "PRRT_kwDOS6i-_86KixXq",
    )
}
#[test]
fn validator_rejects_has_not_been_no_change_rationale() -> TestResult {
    assert_rejects_thread_rationale(
        "Review response: addressed current head. Accepted no-change rationale has not been documented for thread PRRT_kwDOS6i-_86Km2Tf.\n",
        "PRRT_kwDOS6i-_86Km2Tf",
    )
}
#[test]
fn validator_rejects_missing_no_change_rationale_labels() -> TestResult {
    assert_rejects_thread_rationale(
        "Review response: addressed current head. Accepted no-change rationale is missing for thread PRRT_kwDOS6i-_86KnflQ.\n",
        "PRRT_kwDOS6i-_86KnflQ",
    )?;
    assert_rejects_thread_rationale(
        "Review response: addressed current head. Accepted no-change rationale hasn't been documented for thread PRRT_kwDOS6i-_86KnflQ.\n",
        "PRRT_kwDOS6i-_86KnflQ",
    )
}
#[test]
fn validator_treats_review_comments_as_review_response() -> TestResult {
    assert_requires_threads("Addressed Codex review comments on the current head.\n")?;
    assert_requires_threads("Addressed the Codex review comment on the current head.\n")
}
#[test]
fn validator_treats_review_suggestions_as_review_response() -> TestResult {
    assert_requires_threads("Applied Codex review suggestions on the current head.\n")?;
    assert_requires_threads("Addressed the Codex review suggestion on the current head.\n")
}
#[test]
fn validator_treats_codex_review_feedback_as_review_response() -> TestResult {
    assert_requires_threads("Codex review:\n- Fixed the requested changes.\n")?;
    assert_requires_threads("Updated actionable Codex feedback.\n")?;
    assert_requires_threads("Handled actionable Codex feedback.\n")
}
#[test]
fn validator_treats_resolved_review_comments_as_review_response() -> TestResult {
    assert_requires_threads("Implemented Codex review comments on the current head.\n")
}
#[test]
fn validator_treats_present_tense_review_actions_as_review_response() -> TestResult {
    assert_requires_threads("Review response: this fixes the Codex review feedback.\n")?;
    assert_requires_threads("This resolves the review comments.\n")
}
#[test]
fn validator_preserves_review_feedback_context_across_section_breaks() -> TestResult {
    assert_requires_threads(
        "Review feedback:\n- Addressed all requested changes on the current head.\n",
    )
}
#[test]
fn validator_preserves_review_feedback_context_across_all_bullets() -> TestResult {
    assert_requires_threads("Review feedback:\n- Verification rerun.\n- Fixed the Codex comment.\n")
}
#[test]
fn validator_allows_unresolved_status_without_action_word_match() -> TestResult {
    assert_handoff_succeeds(
        "Review feedback: unresolved thread remains; this lane is not complete.\n",
        NORMAL_OPEN_PR_STATE,
    )
}
#[test]
fn validator_rejects_missing_review_thread_page_info() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. PR ready for parent handoff.\n",
        unresolved_review_thread_pr_state().replace("\"pageInfo\":{\"hasNextPage\":false},", ""),
        "pagination",
    )
}
#[test]
fn validator_rejects_partial_review_thread_evidence() -> TestResult {
    assert_handoff_fails(
        "Review response: addressed current head. PR ready for parent handoff.\n",
        partial_review_thread_pr_state(),
        "incomplete reviewThreads.nodes",
    )
}
#[test]
fn validator_allows_clean_codex_review_with_unrelated_fix_without_threads() -> TestResult {
    assert_handoff_succeeds(
        "Codex review passed. Fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
    )
}
#[test]
fn validator_allows_no_review_feedback_with_unrelated_fix_without_threads() -> TestResult {
    assert_handoff_succeeds(
        "Review feedback: none from Codex. Fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
    )
}
#[test]
fn validator_limits_no_feedback_negation_to_current_clause() -> TestResult {
    assert_requires_threads(
        "No review feedback was left unresolved. Review response: fixed the Codex review feedback.\n",
    )?;
    assert_requires_threads(
        "Review response: no review feedback was left unresolved, fixed the Codex review feedback.\n",
    )
}
#[test]
fn validator_allows_comma_separated_no_review_feedback_with_unrelated_fix() -> TestResult {
    assert_handoff_succeeds(
        "Review feedback: none from Codex, fixed the failing test.\n",
        NORMAL_OPEN_PR_STATE,
    )
}
#[test]
fn validator_limits_negation_to_matched_review_action() -> TestResult {
    assert_requires_threads("Review feedback: did not change the API but fixed the review comment.")
}
#[test]
fn validator_rejects_unresolved_outdated_review_thread_after_response() -> TestResult {
    assert_handoff_fails(
        "Review response: fixed the Codex review feedback on the current head.\n",
        outdated_unresolved_review_thread_pr_state(),
        "PRRT_kwDOOutdated",
    )
}
#[test]
fn validator_allows_rationale_after_unrelated_no_blockers_summary() -> TestResult {
    assert_handoff_succeeds(
        "No blockers. Accepted no-change rationale documented for thread PRRT_kwDOExample.\n",
        unresolved_review_thread_pr_state(),
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
fn assert_handoff_fails(handoff: &str, pr_state: impl AsRef<str>, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, pr_state)?;
    assert_failure_contains(&output, needle);
    Ok(())
}
fn assert_requires_threads(handoff: &str) -> TestResult {
    assert_handoff_fails(handoff, NORMAL_OPEN_PR_STATE, "reviewThreads.nodes")
}
fn assert_rejects_thread_rationale(handoff: &str, id: &str) -> TestResult {
    assert_handoff_fails(handoff, unresolved_review_thread_with_id_pr_state(id), id)
}
fn assert_handoff_succeeds(handoff: &str, pr_state: impl AsRef<str>) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, pr_state)?;
    assert_success(&output);
    Ok(())
}
fn assert_failure_contains(output: &std::process::Output, needle: &str) {
    assert!(
        !output.status.success(),
        "validator should fail\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(needle),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "validator should pass\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
fn outdated_unresolved_review_thread_pr_state() -> String {
    unresolved_review_thread_with_id_pr_state("PRRT_kwDOOutdated")
        .replace("\"isOutdated\": false", "\"isOutdated\": true")
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
fn partial_review_thread_pr_state() -> String {
    unresolved_review_thread_with_id_pr_state("PRRT_kwDOPartial").replace(
        "\"pageInfo\":{\"hasNextPage\":false},",
        "\"pageInfo\":{\"hasNextPage\":true,\"endCursor\":\"cursor\"},",
    )
}
fn unresolved_review_thread_with_id_pr_state(id: &str) -> String {
    format!(
        r#"{{
        "number": 134,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {{"pageInfo":{{"hasNextPage":false}},
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
