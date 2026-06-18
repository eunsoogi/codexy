use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_negated_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. No accepted no-change rationale documented for thread PRRT_kwDOExample.\n",
        unresolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject negated no-change rationale text\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOExample"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_post_label_negated_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. Accepted no-change rationale was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        unresolved_review_thread_with_id_pr_state("PRRT_kwDOS6i-_86KixXq"),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject post-label negated no-change rationale text\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOS6i-_86KixXq"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_punctuated_post_label_negated_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. Accepted no-change rationale: was not documented for thread PRRT_kwDOS6i-_86KixXq.\n",
        unresolved_review_thread_with_id_pr_state("PRRT_kwDOS6i-_86KixXq"),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject punctuated post-label negated rationale text\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOS6i-_86KixXq"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_treats_review_comments_as_review_response() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Addressed Codex review comments on the current head.\n",
        r#"{"number":134,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should require reviewThreads.nodes for addressed review comments\nstdout:\n{}\nstderr:\n{}",
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
fn validator_rejects_incomplete_review_thread_evidence() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: addressed current head. PR ready for parent handoff.\n",
        r#"{
            "number": 134,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "reviewDecision": "APPROVED",
            "reviewThreads": {"nodes": [{}]}
        }"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should fail closed on incomplete review thread evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("incomplete reviewThreads.nodes"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_clean_codex_review_with_unrelated_fix_without_threads() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed. Fixed the failing test.\n",
        r#"{"number":134,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not treat unrelated fixes as review feedback responses\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_unresolved_outdated_review_thread_after_response() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review feedback on the current head.\n",
        outdated_unresolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should require resolution for addressed outdated review threads\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOOutdated"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: impl AsRef<str>) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state.as_ref())?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
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
