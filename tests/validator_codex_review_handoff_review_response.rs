use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_accepts_later_nonempty_body_codex_approval_review() -> TestResult {
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
            "reviews":[{"body":"Reviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","state":"APPROVED","commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"},"author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should accept nonempty-body Codex approval review state as output\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unhandled_top_level_codex_actionable_output() -> TestResult {
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
                "createdAt":"2026-06-22T12:45:06Z",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Found an actionable issue in the validator.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject readiness claims with unhandled actionable top-level Codex output",
        "actionable Codex review output",
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_none_valued_codex_suggestion_sections() -> TestResult {
    for body in [
        "Suggestions: none\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
        "Actionable issues: none\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
    ] {
        let output = validate_handoff_with_pr_state(
            "Codex review passed on the current head. PR is merge-ready.\n",
            &format!(
                r#"{{
                    "number":156,
                    "state":"OPEN",
                    "isDraft":false,
                    "mergeStateStatus":"CLEAN",
                    "reviewDecision":"APPROVED",
                    "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
                    "comments":[{{
                        "body":"@codex review",
                        "author":{{"login":"eunsoogi"}},
                        "createdAt":"2026-06-22T12:45:06Z",
                        "reactionGroups":[{{"content":"EYES","users":{{"totalCount":1}}}}]
                    }}],
                    "latestReviews":[{{
                        "body":{},
                        "author":{{"login":"chatgpt-codex-connector"}},
                        "submittedAt":"2026-06-22T12:50:03Z"
                    }}],
                    "reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}
                }}"#,
                serde_json::to_string(body)?
            ),
        )?;
        assert!(
            output.status.success(),
            "validator should accept none-valued Codex section body: {body}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_none_prefixed_actionable_section_values() -> TestResult {
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
                "createdAt":"2026-06-22T12:45:06Z",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "latestReviews":[{
                "body":"Suggestion: none of the validator tests cover non-empty none-prefixed label values.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-06-22T12:50:03Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject actionable Codex section values that only start with none",
        "actionable Codex review output",
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
