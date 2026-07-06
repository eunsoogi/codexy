use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_quoted_codex_review_boilerplate_in_readiness_handoff() -> TestResult {
    for footer in [
        r#"Comment "@codex review" to request another review."#,
        r#"Comment '@codex review' to request another review."#,
        r#"Comment `@codex review` to request another review."#,
        r#"> Comment "@codex review" to request another review."#,
        r#"- Comment "@codex review" to request another review."#,
        r#"- [x] Comment "@codex review" to request another review."#,
        r#"- Comment "@codex review"."#,
        r#"> - Comment "@codex review"."#,
        r#"> - [x] Comment "@codex review"."#,
    ] {
        let output = validate_handoff_with_pr_state(
            &format!(
                r#"Codex review completed. PR is merge-ready. Maintainer requested leave-open for handoff.

Connector evidence:
Reviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`
{footer}
"#
            ),
            current_head_output_pr_state(),
        )?;
        assert!(
            output.status.success(),
            "validator should not treat quoted Codex connector boilerplate as a fresh review request\nfooter: {footer}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_suffixed_quoted_footer_as_fresh_request() -> TestResult {
    for request in [
        r#"Comment "@codex review" to request another review now."#,
        r#"Comment "@codex review" to request another review on the current head."#,
        r#"- Comment "@codex review" on the current head."#,
        r#"- [x] Comment "@codex review" on the current head."#,
        r#"> - Comment "@codex review" on the current head."#,
    ] {
        let output = validate_handoff_with_pr_state(request, current_head_output_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should preserve duplicate-review guard for suffixed review requests\nrequest: {request}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("current-head Codex review activity blocks fresh Codex review requests"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_negative_codex_review_request_status_labels() -> TestResult {
    for status in [
        "Codex review request: none yet.\n",
        "Codex review request? none yet.\n",
        "- Codex review request: none yet.\n",
        "- [ ] Codex review request: none yet.\n",
        "Codex review request: not requested.\n",
        "Current-head Codex review request: none.\n",
        "@codex review request: false.\n",
    ] {
        let output = validate_handoff_with_pr_state(status, unresolved_thread_pr_state())?;
        assert!(
            output.status.success(),
            "validator should not treat negative request status labels as fresh review requests\nstatus: {status}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_fresh_codex_review_request_after_stale_activity() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Request exactly one fresh Codex review now.\n",
        stale_codex_activity_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "validator should allow one fresh review when prior request and output are stale\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_pull_request_readiness_after_codex_review() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Pull request is ready after Codex review completed. Maintainer requested leave-open for handoff.\n",
        current_head_output_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "validator should not treat pull request readiness nouns as fresh Codex review requests\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

fn current_head_output_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content":"EYES","users":{"totalCount":1}}]
        }],
        "reviews": [{
            "body":"Review completed.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`\n\nComment \"@codex review\" to request another review.",
            "state": "COMMENTED",
            "author":{"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z",
            "commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"}
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}

fn stale_codex_activity_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "comments": [{
            "body": "@codex review",
            "author": {"login": "eunsoogi"},
            "createdAt": "2026-06-22T12:45:06Z",
            "reactionGroups": [{"content":"EYES","users":{"totalCount":1}}]
        }],
        "reviews": [{
            "body":"Review completed.\n\nReviewed commit: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`",
            "state": "COMMENTED",
            "author":{"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z",
            "commit":{"oid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
