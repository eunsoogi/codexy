use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_review_response_handoff_with_unresolved_thread() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. PR ready for parent handoff.\n",
        unresolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject addressed review feedback with unresolved review threads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unresolved review thread"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_review_response_handoff_with_accepted_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOExample. Preventive adjacent review: focused regression coverage exercises adjacent workflow variants in the touched helper family.\n",
        unresolved_review_thread_pr_state(),
    )?;

    assert!(
        output.status.success(),
        "validator should allow explicit accepted no-change rationale\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_no_change_rationale_that_does_not_cover_each_thread() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOAccepted.\n",
        two_unresolved_review_threads_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject unresolved threads without per-thread accepted rationale\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOMissing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_thread_mention_outside_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. Accepted no-change rationale documented for thread PRRT_kwDOAccepted, fixed PRRT_kwDOMissing.\n",
        two_unresolved_review_threads_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should tie no-change rationale to the same unresolved thread\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOMissing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_path_only_no_change_rationale_for_multiple_threads() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. Accepted no-change rationale documented for plugins/codexy/skills/git-workflow/SKILL.md.\n",
        two_same_path_unresolved_review_threads_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should require thread-specific id or URL rationale, not only a shared path\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOSamePathOne"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_review_response_handoff_without_review_thread_state() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed and verified current head. PR ready for parent handoff.\n",
        r#"{"number":133,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should require reviewThreads.nodes evidence for review responses\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("reviewThreads.nodes"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_clean_codex_review_verification_without_review_threads() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review verified on the latest head. No review feedback was addressed in this handoff.\n",
        r#"{"number":134,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not require reviewThreads.nodes for clean review verification\nstdout:\n{}\nstderr:\n{}",
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

fn unresolved_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOExample",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {
                        "nodes": [
                            {
                                "url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r1",
                                "body": "Please resolve addressed review threads after verifying current head."
                            }
                        ]
                    }
                }
            ]
        }
    }"#
}

fn two_unresolved_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOAccepted",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r1"}]}
                },
                {
                    "id": "PRRT_kwDOMissing",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/qa/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r2"}]}
                }
            ]
        }
    }"#
}

fn two_same_path_unresolved_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOSamePathOne",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r10"}]}
                },
                {
                    "id": "PRRT_kwDOSamePathTwo",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "plugins/codexy/skills/git-workflow/SKILL.md",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r11"}]}
                }
            ]
        }
    }"#
}
