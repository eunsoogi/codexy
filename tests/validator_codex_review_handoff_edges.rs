use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_colon_label_before_merge_ready_claim() -> TestResult {
    let output =
        validate_handoff_with_pr_state("No blockers: PR is merge-ready.\n", eyes_only_pr_state())?;
    assert_rejected_eyes_only(
        &output,
        "validator should treat colon-delimited labels as clause boundaries",
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_later_inline_codex_review_comment() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r##"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-06-22T12:45:06Z",
                "reactionGroups":[{
                    "content":"EYES",
                    "users":{"totalCount":1}
                }]
            }],
            "reviewThreads":{"nodes":[{
                "isResolved":true,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff.rs",
                "comments":{"nodes":[{
                    "body":"Use the existing helper here.",
                    "url":"https://github.com/eunsoogi/codexy/pull/165#discussion_r3453572726",
                    "author":{"login":"chatgpt-codex-connector"},
                    "createdAt":"2026-06-22T12:50:03Z"
                }]}
            }]}
        }"##,
    )?;
    assert!(
        output.status.success(),
        "validator should accept later inline Codex comments as review output\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unresolved_inline_codex_review_comment() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r##"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-06-22T12:45:06Z",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "reviewThreads":{"nodes":[{
                "isResolved":false,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff.rs",
                "comments":{"nodes":[{
                    "body":"Use the existing helper here.",
                    "url":"https://github.com/eunsoogi/codexy/pull/165#discussion_r3454309679",
                    "author":{"login":"chatgpt-codex-connector"},
                    "createdAt":"2026-06-22T12:50:03Z"
                }]}
            }]}
        }"##,
    )?;
    assert_rejected_codex_thread(
        &output,
        "validator should reject readiness claims with unresolved Codex review threads",
    );
    Ok(())
}

#[test]
fn validator_cli_allows_non_codex_review_approval_wait_state() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Maintainer review approved; Codex review is not complete.\n",
        eyes_only_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "validator should not treat unrelated review approval as Codex readiness\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_later_empty_body_codex_approval_review() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review approved on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],
            "reviews":[{"body":"","state":"APPROVED","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}]
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should accept empty-body Codex approval review state as output\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unchecked_maintainer_override_with_eyes_only_review() -> TestResult {
    for marker in [
        "- [ ]", "* [ ]", "+ [ ]", "-  [ ]", "*  [ ]", "+  [ ]", "-\t[ ]", "*\t[ ]", "+\t[ ]",
        "1. [ ]", "2.  [ ]", "3.\t[ ]",
    ] {
        let output = validate_handoff_with_pr_state(
            &format!(
                "{marker} Maintainer override: yes\nCodex review passed. PR is merge-ready.\n"
            ),
            eyes_only_pr_state(),
        )?;
        assert_rejected_eyes_only(
            &output,
            "validator should ignore unchecked maintainer override checklist items",
        );
    }
    Ok(())
}

fn eyes_only_pr_state() -> &'static str {
    r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}]}"#
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

fn assert_rejected_codex_thread(output: &std::process::Output, message: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unresolved Codex review thread"),
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
