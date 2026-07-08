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
        "Codex review passed on the current head. Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. PR is merge-ready.\n",
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
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{
                "id":"PRRT_kwDOS6i-_86LUQLC",
                "isResolved":true,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff.rs",
                "comments":{"nodes":[{
                    "body":"Use the existing helper here.",
                    "url":"https://github.com/eunsoogi/codexy/pull/165#discussion_r3453572726",
                    "commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"},
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
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{
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
    assert_rejected_with_stderr(
        &output,
        "validator should reject readiness claims with unresolved Codex review threads",
        "unresolved Codex review thread",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_without_complete_review_thread_evidence() -> TestResult {
    for review_threads in [
        "",
        r#","reviewThreads":{"pageInfo":{"hasNextPage":true},"nodes":[]}"#,
    ] {
        let output = validate_handoff_with_pr_state(
            "Codex review passed on the current head. PR is merge-ready.\n",
            &format!(
                r#"{{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{{"body":"@codex review","author":{{"login":"eunsoogi"}},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{{"content":"EYES","users":{{"totalCount":1}}}}]}},{{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","author":{{"login":"chatgpt-codex-connector"}},"createdAt":"2026-06-22T12:50:03Z"}}]{review_threads}}}"#
            ),
        )?;
        assert_rejected_with_stderr(
            &output,
            "validator should require complete reviewThreads evidence before readiness",
            "reviewThreads.nodes",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_codex_output_for_prior_head() -> TestResult {
    for (pr_state, message) in [
        (
            r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","commit":{"oid":"aaaaaaaaaa"},"reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],"latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaa`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
            "validator should reject Codex output reviewed on an older head",
        ),
        (
            r#"{"number":156,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","commit":{"oid":"aaaaaaaaaa"},"reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],"latestReviews":[{"body":"Didn't find any major issues.","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
            "validator should reject Codex output without current-head commit evidence",
        ),
    ] {
        let output = validate_handoff_with_pr_state(
            "Codex review passed on the current head. PR is merge-ready.\n",
            pr_state,
        )?;
        assert_rejected_with_stderr(&output, message, "current-head Codex review output");
    }
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
        "Codex review approved on the current head. Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z","reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]}],
            "reviews":[{"body":"","state":"APPROVED","commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"},"author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
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
        "1. [ ]", "2.  [ ]", "3.\t[ ]", "1) [ ]", "2)  [ ]", "3)\t[ ]",
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

fn assert_rejected_with_stderr(output: &std::process::Output, message: &str, expected: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
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
