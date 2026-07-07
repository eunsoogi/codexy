use std::{path::Path, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_eyes_only_codex_review_as_merge_ready() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        eyes_only_pr_state(),
    )?;
    assert_rejected_eyes_only(
        &output,
        "validator should reject eyes-only Codex review evidence as merge-ready",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_stale_codex_output_before_current_eyes_request() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector[bot]"}
            },{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "reactionGroups":[{
                    "content":"EYES",
                    "users":{"totalCount":1}
                }]
            }]
        }"#,
    )?;
    assert_rejected_eyes_only(
        &output,
        "validator should reject stale Codex output before a current eyes-only request",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_stale_codex_review_from_separate_array() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-06-22T10:00:00Z",
                "reactionGroups":[{
                    "content":"EYES",
                    "users":{"totalCount":1}
                }]
            }],
            "reviews":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector[bot]"},
                "submittedAt":"2026-06-22T09:00:00Z"
            }]
        }"#,
    )?;
    assert_rejected_eyes_only(
        &output,
        "validator should reject stale Codex review output from another evidence array",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_untimestamped_review_thread_output_after_eyes_request() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T10:00:00Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],"reviewThreads":{"nodes":[{"comments":{"nodes":[{"body":"Didn't find any major issues.","author":{"login":"chatgpt-codex-connector[bot]"}}]}}]}}"#,
    )?;
    assert_rejected_eyes_only(
        &output,
        "validator should reject untimestamped review-thread output after an eyes request",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_codex_progress_comment_after_eyes_request() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-06-22T10:00:00Z",
                "reactionGroups":[{
                    "content":"EYES",
                    "users":{"totalCount":1}
                }]
            },{
                "body":"Review started; I will post suggestions when complete.",
                "author":{"login":"chatgpt-codex-connector[bot]"},
                "createdAt":"2026-06-22T10:01:00Z"
            }]
        }"#,
    )?;
    assert_rejected_eyes_only(
        &output,
        "validator should reject connector progress comments as completed review output",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_generic_merge_ready_claim_with_eyes_only_review() -> TestResult {
    for handoff in [
        "This branch is PR-ready. This lane is not complete until merge.\n",
        "PR is not merge-ready while waiting. After this gate, PR is merge-ready.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, eyes_only_pr_state())?;
        assert_rejected_eyes_only(
            &output,
            "validator should reject generic merge-ready claims with eyes-only Codex review evidence",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_allows_negated_wait_state_with_eyes_only_review() -> TestResult {
    for handoff in [
        "Codex review is not complete. Waiting for review output.\n",
        "PR is not merge-ready because Codex review is pending.\n",
        "PR is not currently merge-ready because Codex review is pending.\n",
        "PR isn't merge-ready because Codex review is pending.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, eyes_only_pr_state())?;
        assert!(output.status.success());
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_maintainer_override_with_eyes_only_review() -> TestResult {
    for handoff in [
        "No maintainer override provided. Codex review passed. PR is merge-ready.\n",
        "No maintainer override was provided. Codex review passed. PR is merge-ready.\n",
        "Maintainer override: none. Codex review passed. PR is merge-ready.\n",
        "No maintainer accepted proceeding without Codex review. Codex review passed. PR is merge-ready.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, eyes_only_pr_state())?;
        assert_rejected_eyes_only(
            &output,
            "validator should reject negated maintainer override claims",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_affirmative_maintainer_override() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Maintainer override: yes. Codex review passed. PR is merge-ready.\n",
        eyes_only_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_later_completed_compact_codex_review() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],
            "latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should accept completed compact Codex review output after the eyes request\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn eyes_only_pr_state() -> &'static str {
    r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#
}

fn assert_rejected_eyes_only(output: &std::process::Output, message: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("eyes-only Codex review request"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
