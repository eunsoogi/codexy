use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_unacknowledged_codex_review_request() -> TestResult {
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
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2001",
                "reactionGroups":[]
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject readiness while a Codex review request lacks current-head output",
        "current-head Codex review output is required",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unresolved_thread_without_author_identity() -> TestResult {
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
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2002",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{
                "id":"PRRT_kwDOS6i-_86LdDFD",
                "isResolved":false,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff_events.rs",
                "comments":{"nodes":[{
                    "url":"https://github.com/eunsoogi/codexy/pull/165#discussion_r3456760126"
                }]}
            }]}
        }"##,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject unresolved review-thread evidence without comment identity",
        "review thread",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unresolved_thread_with_null_author_identity() -> TestResult {
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
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2003",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{
                "id":"PRRT_kwDOS6i-_86LdccI",
                "isResolved":false,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff_events.rs",
                "comments":{"nodes":[{
                    "url":"https://github.com/eunsoogi/codexy/pull/165#discussion_r3456905132",
                    "author":null
                }]}
            }]}
        }"##,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject unresolved review-thread evidence with null comment identity",
        "review thread",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_codex_readiness_without_head_ref_oid() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2004",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should require headRefOid before current-head Codex readiness claims",
        "headRefOid",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_codex_readiness_with_blank_head_ref_oid() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":156,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "reviewDecision":"APPROVED",
            "headRefOid":"   ",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2005",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject blank headRefOid before current-head Codex readiness claims",
        "headRefOid",
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_connector_output_with_review_request_footer() -> TestResult {
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
                "author":{"login":"eunsoogi"},"createdAt":"2026-06-22T12:45:06Z",
                "url":"https://github.com/eunsoogi/codexy/pull/156#issuecomment-2006",
                "reactionGroups":[]
            }],
            "latestReviews":[{
                "body":"Review completed.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`\n\nComment \"@codex review\" to request another review.",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should accept later connector output even when boilerplate mentions @codex review\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
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
